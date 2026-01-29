use super::action::{execute_action, parse_action, ActionError};
use super::queue::{QueueFailureMode, QueueManager};
use super::state::{AgentStateManager, AgentStatus};
use crate::capture::capture_primary_screen;
use crate::config::Config;
use crate::llm::{AnthropicProvider, LlmProvider, OllamaProvider, OpenAIProvider, OpenRouterProvider};
use serde::Serialize;
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
    #[error("Queue item failed: {0}")]
    QueueItemFailed(String),
}

#[derive(Clone, Serialize)]
pub struct QueueProgressEvent {
    pub current_index: usize,
    pub total: usize,
    pub current_instruction: String,
    pub status: String,
}

pub struct AgentLoop {
    state: AgentStateManager,
    config: Config,
    app_handle: AppHandle,
    queue: Option<QueueManager>,
}

impl AgentLoop {
    pub fn new(state: AgentStateManager, config: Config, app_handle: AppHandle) -> Self {
        Self {
            state,
            config,
            app_handle,
            queue: None,
        }
    }

    pub fn with_queue(mut self, queue: QueueManager) -> Self {
        self.queue = Some(queue);
        self
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

            match execute_action(&action, confirm_dangerous) {
                Ok(result) => {
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

    pub async fn run_queue(&self) -> Result<(), LoopError> {
        let queue = match &self.queue {
            Some(q) => q,
            None => return Err(LoopError::NoProvider),
        };

        let failure_mode = QueueFailureMode::from(self.config.general.queue_failure_mode.as_str());
        let queue_delay = Duration::from_millis(self.config.general.queue_delay_ms as u64);

        queue.set_processing(true).await;

        let total = queue.total_count().await;
        self.state.set_queue_info(0, total, true).await;

        loop {
            // Check if should stop
            if self.state.should_stop() {
                queue.set_processing(false).await;
                self.state.set_status(AgentStatus::Idle).await;
                self.state.set_queue_info(0, 0, false).await;
                self.emit_state_update().await;
                self.emit_queue_update().await;
                return Err(LoopError::Stopped);
            }

            // Get next pending item
            let item = match queue.get_next().await {
                Some(item) => item,
                None => {
                    // No more items to process
                    queue.set_processing(false).await;
                    self.state.set_status(AgentStatus::Completed).await;
                    self.state.set_queue_info(total, total, false).await;
                    self.emit_state_update().await;
                    self.emit_queue_update().await;
                    return Ok(());
                }
            };

            let current_index = queue.current_index().await;
            let instruction = item.instruction.clone();

            // Emit queue progress
            let _ = self.app_handle.emit(
                "queue-item-started",
                QueueProgressEvent {
                    current_index,
                    total,
                    current_instruction: instruction.clone(),
                    status: "running".to_string(),
                },
            );

            // Mark as running
            queue.mark_current_running().await;
            self.state.update_queue_progress(current_index + 1, total).await;
            self.emit_state_update().await;
            self.emit_queue_update().await;

            // Run the instruction
            let result = self.run(instruction.clone()).await;

            match result {
                Ok(()) => {
                    queue.mark_current_completed(Some("Completed".to_string())).await;
                    let _ = self.app_handle.emit(
                        "queue-item-completed",
                        QueueProgressEvent {
                            current_index,
                            total,
                            current_instruction: instruction,
                            status: "completed".to_string(),
                        },
                    );
                }
                Err(LoopError::Stopped) => {
                    // User stopped - exit immediately
                    queue.set_processing(false).await;
                    self.emit_queue_update().await;
                    return Err(LoopError::Stopped);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    queue.mark_current_failed(error_msg.clone()).await;

                    let _ = self.app_handle.emit(
                        "queue-item-failed",
                        QueueProgressEvent {
                            current_index,
                            total,
                            current_instruction: instruction,
                            status: "failed".to_string(),
                        },
                    );

                    match failure_mode {
                        QueueFailureMode::Stop => {
                            queue.set_processing(false).await;
                            self.state.set_queue_info(current_index + 1, total, false).await;
                            self.emit_state_update().await;
                            self.emit_queue_update().await;
                            return Err(LoopError::QueueItemFailed(error_msg));
                        }
                        QueueFailureMode::Continue => {
                            // Continue to next item
                        }
                    }
                }
            }

            // Advance to next item
            if !queue.advance().await {
                // No more items
                queue.set_processing(false).await;
                self.state.set_status(AgentStatus::Completed).await;
                self.state.set_queue_info(total, total, false).await;
                self.emit_state_update().await;
                self.emit_queue_update().await;
                return Ok(());
            }

            // Delay between queue items
            if queue_delay.as_millis() > 0 {
                sleep(queue_delay).await;
            }
        }
    }

    async fn emit_state_update(&self) {
        let state = self.state.get_state().await;
        let _ = self.app_handle.emit("agent-state", state);
    }

    async fn emit_queue_update(&self) {
        if let Some(queue) = &self.queue {
            let queue_state = queue.get_state().await;
            let _ = self.app_handle.emit("queue-update", queue_state);
        }
    }
}
