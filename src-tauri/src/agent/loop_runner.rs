use super::action::{execute_action, parse_action, ActionError};
use super::recovery::{
    classify_capture_error, classify_llm_error, retry_with_policy, ErrorClassification,
    RetryPolicy,
};
use super::state::{AgentStateManager, AgentStatus};
use crate::capture::{capture_primary_screen, CaptureError, Screenshot};
use crate::config::Config;
use crate::llm::{
    AnthropicProvider, LlmError, LlmProvider, OllamaProvider, OpenAIProvider, OpenRouterProvider,
    TokenMetrics,
};
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
    #[error("Too many consecutive errors: {0}")]
    TooManyErrors(u32),
}

const MAX_CONSECUTIVE_ERRORS: u32 = 3;

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

        self.state.start(instruction.clone(), max_iterations).await;
        self.emit_state_update().await;

        loop {
            // Check if should stop
            if self.state.should_stop() {
                self.state.set_status(AgentStatus::Idle).await;
                self.emit_state_update().await;
                return Err(LoopError::Stopped);
            }

            // Check consecutive error limit
            let consecutive_errors = self.state.get_consecutive_errors().await;
            if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                self.state
                    .set_error(format!(
                        "Too many consecutive errors ({})",
                        consecutive_errors
                    ))
                    .await;
                self.emit_state_update().await;
                return Err(LoopError::TooManyErrors(consecutive_errors));
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

            // Capture screenshot with retry
            let screenshot = match self.capture_with_retry().await {
                Ok(s) => s,
                Err(e) => {
                    self.state.increment_consecutive_errors().await;
                    self.state.set_error(e.to_string()).await;
                    self.emit_state_update().await;
                    return Err(e.into());
                }
            };

            // Send to LLM with retry
            let llm_result = self
                .send_to_llm_with_retry(&provider, &instruction, &screenshot)
                .await;

            let (response, metrics) = match llm_result {
                Ok((resp, met)) => {
                    // Reset consecutive errors on success
                    self.state.reset_consecutive_errors().await;
                    (resp, met)
                }
                Err(e) => {
                    self.state.increment_consecutive_errors().await;
                    self.state.set_error(e.to_string()).await;
                    self.emit_state_update().await;

                    // Check if we should continue or bail
                    let classification = classify_llm_error(&e);
                    if matches!(classification, ErrorClassification::Fatal) {
                        return Err(e.into());
                    }

                    // Non-fatal error, continue to next iteration
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Update metrics
            self.state
                .update_metrics(
                    metrics.tokens_per_second(),
                    metrics.input_tokens,
                    metrics.output_tokens,
                )
                .await;
            self.emit_state_update().await;

            // Parse action - on parse error, send feedback to LLM
            let action = match parse_action(&response) {
                Ok(a) => a,
                Err(parse_err) => {
                    self.state.increment_consecutive_errors().await;

                    // Emit parse error feedback
                    let _ = self.app_handle.emit(
                        "parse-error",
                        format!("Failed to parse LLM response: {}", parse_err),
                    );

                    // Continue to next iteration - the LLM will see the error
                    // in subsequent iterations via conversation context (when available)
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };

            self.state
                .set_last_action(serde_json::to_string(&action).unwrap_or_default())
                .await;
            self.emit_state_update().await;

            match execute_action(&action, confirm_dangerous) {
                Ok(result) => {
                    // Reset consecutive errors on successful action
                    self.state.reset_consecutive_errors().await;

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
                    self.state.increment_consecutive_errors().await;
                    self.state.set_error(e.to_string()).await;
                    self.emit_state_update().await;

                    // Action errors are generally not retryable
                    return Err(e.into());
                }
            }

            // Small delay between iterations
            sleep(Duration::from_millis(500)).await;
        }
    }

    /// Capture screenshot with retry logic
    async fn capture_with_retry(&self) -> Result<Screenshot, CaptureError> {
        let policy = RetryPolicy::for_screenshots();

        let result = retry_with_policy(&policy, classify_capture_error, || async {
            capture_primary_screen()
        })
        .await;

        if result.attempts > 1 {
            self.state.increment_retry().await;
            let _ = self
                .app_handle
                .emit("retry-info", format!("Screenshot captured after {} attempts", result.attempts));
        }

        result.result
    }

    /// Send to LLM with retry logic
    async fn send_to_llm_with_retry(
        &self,
        provider: &Box<dyn LlmProvider>,
        instruction: &str,
        screenshot: &Screenshot,
    ) -> Result<(String, TokenMetrics), LlmError> {
        let policy = RetryPolicy::for_llm_calls();
        let mut attempts = 0;

        loop {
            attempts += 1;

            // Check if should stop before each attempt
            if self.state.should_stop() {
                return Err(LlmError::ApiError("Agent stopped".to_string()));
            }

            // Create callback for chunk streaming
            let app_handle = self.app_handle.clone();
            let on_chunk: Box<dyn Fn(&str) + Send + Sync> = Box::new(move |chunk: &str| {
                let _ = app_handle.emit("llm-chunk", chunk.to_string());
            });

            let result = provider
                .send_with_image(
                    instruction,
                    &screenshot.base64,
                    screenshot.width,
                    screenshot.height,
                    on_chunk,
                )
                .await;

            match result {
                Ok((response, metrics)) => {
                    if attempts > 1 {
                        self.state.increment_retry().await;
                        let _ = self.app_handle.emit(
                            "retry-info",
                            format!("LLM call succeeded after {} attempts", attempts),
                        );
                    }
                    return Ok((response, metrics));
                }
                Err(error) => {
                    let classification = classify_llm_error(&error);

                    // Check if we should retry
                    let should_retry = match classification {
                        ErrorClassification::Fatal => false,
                        ErrorClassification::Retryable | ErrorClassification::RateLimited { .. } => {
                            attempts <= policy.max_retries
                        }
                    };

                    if !should_retry {
                        return Err(error);
                    }

                    // Update state to show we're retrying
                    self.state.set_status(AgentStatus::Retrying).await;
                    self.emit_state_update().await;

                    // Calculate delay
                    let delay = match classification {
                        ErrorClassification::RateLimited { wait_seconds } => {
                            let _ = self.app_handle.emit(
                                "retry-info",
                                format!("Rate limited, waiting {} seconds", wait_seconds),
                            );
                            Duration::from_secs(wait_seconds)
                        }
                        _ => {
                            let delay = policy.delay_for_attempt(attempts);
                            let _ = self.app_handle.emit(
                                "retry-info",
                                format!(
                                    "LLM error (attempt {}), retrying in {:?}",
                                    attempts, delay
                                ),
                            );
                            delay
                        }
                    };

                    sleep(delay).await;

                    // Restore running status
                    self.state.set_status(AgentStatus::Running).await;
                    self.emit_state_update().await;
                }
            }
        }
    }

    async fn emit_state_update(&self) {
        let state = self.state.get_state().await;
        let _ = self.app_handle.emit("agent-state", state);
    }
}
