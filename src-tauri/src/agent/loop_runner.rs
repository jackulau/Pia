use super::action::{execute_action, execute_action_with_retry, parse_action, ActionError};
use super::retry::RetryContext;
use super::state::{AgentStateManager, AgentStatus};
use crate::capture::capture_primary_screen;
use crate::config::Config;
use crate::llm::{AnthropicProvider, LlmProvider, OllamaProvider, OpenAIProvider, OpenRouterProvider};
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tokio::time::{sleep, Duration};

#[derive(Error, Debug)]
pub enum LoopError {
    #[error("Capture error: {0}")]
    CaptureError(#[from] crate::capture::CaptureError),
    #[error("LLM error: {0}")]
    LlmError(#[from] crate::llm::LlmError),
    #[error("Action error: {0}")]
    ActionError(#[from] ActionError),
    #[error("No provider configured")]
    NoProvider,
    #[error("Agent stopped by user")]
    Stopped,
    #[error("Max iterations reached")]
    MaxIterations,
}

pub struct AgentLoop {
    state: AgentStateManager,
    config: Config,
    app_handle: AppHandle,
}

impl AgentLoop {
    pub fn new(state: AgentStateManager, config: Config, app_handle: AppHandle) -> Self {
        Self {
            state,
            config,
            app_handle,
        }
    }

    fn create_provider(&self) -> Result<Box<dyn LlmProvider>, LoopError> {
        let provider_name = &self.config.general.default_provider;

        match provider_name.as_str() {
            "ollama" => {
                let config = self
                    .config
                    .providers
                    .ollama
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(OllamaProvider::new(
                    config.host.clone(),
                    config.model.clone(),
                )))
            }
            "anthropic" => {
                let config = self
                    .config
                    .providers
                    .anthropic
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(AnthropicProvider::new(
                    config.api_key.clone(),
                    config.model.clone(),
                )))
            }
            "openai" => {
                let config = self
                    .config
                    .providers
                    .openai
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(OpenAIProvider::new(
                    config.api_key.clone(),
                    config.model.clone(),
                )))
            }
            "openrouter" => {
                let config = self
                    .config
                    .providers
                    .openrouter
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(OpenRouterProvider::new(
                    config.api_key.clone(),
                    config.model.clone(),
                )))
            }
            _ => Err(LoopError::NoProvider),
        }
    }

    pub async fn run(&self, instruction: String) -> Result<(), LoopError> {
        let provider = self.create_provider()?;
        let max_iterations = self.config.general.max_iterations;
        let confirm_dangerous = self.config.general.confirm_dangerous_actions;

        // Create retry context from config
        let mut retry_ctx = RetryContext::new(
            self.config.general.max_retries,
            self.config.general.retry_delay_ms,
            self.config.general.enable_self_correction,
        );

        self.state.start(instruction.clone(), max_iterations).await;
        self.emit_state_update().await;

        loop {
            // Check if should stop
            if self.state.should_stop() {
                self.state.set_status(AgentStatus::Idle).await;
                self.emit_state_update().await;
                return Err(LoopError::Stopped);
            }

            // Check iteration limit
            let iteration = self.state.increment_iteration().await;
            if iteration > max_iterations {
                self.state
                    .set_error("Max iterations reached".to_string())
                    .await;
                self.emit_state_update().await;
                return Err(LoopError::MaxIterations);
            }

            // Capture screenshot
            let screenshot = capture_primary_screen()?;

            // Create callback for chunk streaming
            let app_handle = self.app_handle.clone();
            let on_chunk: Box<dyn Fn(&str) + Send + Sync> = Box::new(move |chunk: &str| {
                let _ = app_handle.emit("llm-chunk", chunk.to_string());
            });

            // Send to LLM
            let (response, metrics) = provider
                .send_with_image(
                    &instruction,
                    &screenshot.base64,
                    screenshot.width,
                    screenshot.height,
                    on_chunk,
                )
                .await?;

            // Update metrics
            self.state
                .update_metrics(
                    metrics.tokens_per_second(),
                    metrics.input_tokens,
                    metrics.output_tokens,
                )
                .await;
            self.emit_state_update().await;

            // Parse and execute action
            let action = parse_action(&response)?;
            self.state
                .set_last_action(serde_json::to_string(&action).unwrap_or_default())
                .await;
            self.emit_state_update().await;

            // Use retry-enabled execution if self-correction is enabled
            let execution_result = if self.config.general.enable_self_correction {
                execute_action_with_retry(&action, confirm_dangerous, &mut retry_ctx)
            } else {
                execute_action(&action, confirm_dangerous)
            };

            match execution_result {
                Ok(result) => {
                    // Update retry statistics
                    if result.retry_count > 0 {
                        self.state.update_retry_stats(result.retry_count).await;
                        log::info!("Action succeeded after {} retries", result.retry_count);
                    }
                    self.emit_state_update().await;

                    if result.completed {
                        self.state.complete(result.message).await;
                        self.emit_state_update().await;
                        return Ok(());
                    }
                }
                Err(ActionError::RequiresConfirmation(msg)) => {
                    // Emit confirmation request to frontend
                    let _ = self.app_handle.emit("confirmation-required", msg);
                    self.state.set_status(AgentStatus::Paused).await;
                    self.emit_state_update().await;

                    // Wait for user response (handled externally)
                    // For now, just continue after a delay
                    sleep(Duration::from_secs(5)).await;

                    if self.state.should_stop() {
                        self.state.set_status(AgentStatus::Idle).await;
                        self.emit_state_update().await;
                        return Err(LoopError::Stopped);
                    }

                    self.state.set_status(AgentStatus::Running).await;
                }
                Err(e) => {
                    self.state.set_error(e.to_string()).await;
                    self.emit_state_update().await;
                    return Err(e.into());
                }
            }

            // Small delay between iterations
            sleep(Duration::from_millis(500)).await;
        }
    }

    async fn emit_state_update(&self) {
        let state = self.state.get_state().await;
        let _ = self.app_handle.emit("agent-state", state);
    }
}
