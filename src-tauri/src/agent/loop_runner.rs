use super::action::{execute_action, parse_action, Action, ActionError, ActionResult};
use super::state::{AgentStateManager, AgentStatus};
use crate::capture::capture_primary_screen;
use crate::config::Config;
use crate::llm::{
    AnthropicProvider, LlmProvider, OllamaProvider, OpenAIProvider, OpenRouterProvider, ToolResult,
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

        self.state.start(instruction.clone(), max_iterations).await;
        self.emit_state_update().await;

        // Track pending tool results for next iteration
        let mut pending_tool_results: Option<Vec<ToolResult>> = None;

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

            // Use tool-based API if provider supports it
            if provider.supports_tools() {
                let response = provider
                    .send_with_tools(
                        &instruction,
                        &screenshot.base64,
                        screenshot.width,
                        screenshot.height,
                        pending_tool_results.take(),
                        on_chunk,
                    )
                    .await?;

                // Update metrics
                self.state
                    .update_metrics(
                        response.metrics.tokens_per_second(),
                        response.metrics.input_tokens,
                        response.metrics.output_tokens,
                    )
                    .await;
                self.emit_state_update().await;

                // Process tool uses from the response
                if !response.tool_uses.is_empty() {
                    let mut tool_results = Vec::new();

                    for tool_use in &response.tool_uses {
                        // Convert tool_use input to Action
                        let action_result =
                            self.execute_tool_use(tool_use, confirm_dangerous).await;

                        match action_result {
                            Ok(result) => {
                                // Check if task is complete
                                if result.completed {
                                    self.state.complete(result.message.clone()).await;
                                    self.emit_state_update().await;
                                    return Ok(());
                                }

                                // Create tool result for feedback
                                let tool_result = if result.success {
                                    ToolResult::success(
                                        tool_use.id.clone(),
                                        result.to_tool_result_content(),
                                    )
                                } else {
                                    ToolResult::error(
                                        tool_use.id.clone(),
                                        result.message.unwrap_or_else(|| "Action failed".to_string()),
                                    )
                                };
                                tool_results.push(tool_result);
                            }
                            Err(ActionError::RequiresConfirmation(msg)) => {
                                // Emit confirmation request to frontend
                                let _ = self.app_handle.emit("confirmation-required", msg.clone());
                                self.state.set_status(AgentStatus::Paused).await;
                                self.emit_state_update().await;

                                // Wait for user response
                                sleep(Duration::from_secs(5)).await;

                                if self.state.should_stop() {
                                    self.state.set_status(AgentStatus::Idle).await;
                                    self.emit_state_update().await;
                                    return Err(LoopError::Stopped);
                                }

                                self.state.set_status(AgentStatus::Running).await;

                                // Return error result to LLM
                                tool_results.push(ToolResult::error(
                                    tool_use.id.clone(),
                                    format!("Action requires confirmation: {}", msg),
                                ));
                            }
                            Err(e) => {
                                // Return error result to LLM
                                tool_results.push(ToolResult::error(
                                    tool_use.id.clone(),
                                    format!("Action failed: {}", e),
                                ));
                            }
                        }
                    }

                    // Store tool results for next iteration
                    pending_tool_results = Some(tool_results);
                } else if response.stop_reason.as_deref() == Some("end_turn") {
                    // LLM finished without tool use - check text for complete/error
                    if let Some(text) = &response.text {
                        if let Ok(action) = parse_action(text) {
                            self.state
                                .set_last_action(serde_json::to_string(&action).unwrap_or_default())
                                .await;

                            match execute_action(&action, confirm_dangerous) {
                                Ok(result) => {
                                    if result.completed {
                                        self.state.complete(result.message).await;
                                        self.emit_state_update().await;
                                        return Ok(());
                                    }
                                }
                                Err(e) => {
                                    self.state.set_error(e.to_string()).await;
                                    self.emit_state_update().await;
                                    return Err(e.into());
                                }
                            }
                        }
                    }
                }
            } else {
                // Legacy text-based flow for non-tool providers
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
                        let _ = self.app_handle.emit("confirmation-required", msg);
                        self.state.set_status(AgentStatus::Paused).await;
                        self.emit_state_update().await;

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
            }

            // Small delay between iterations
            sleep(Duration::from_millis(500)).await;
        }
    }

    /// Execute a tool_use request and return the ActionResult
    async fn execute_tool_use(
        &self,
        tool_use: &crate::llm::ToolUse,
        confirm_dangerous: bool,
    ) -> Result<ActionResult, ActionError> {
        // Parse the tool input into an Action
        let action: Action = serde_json::from_value(tool_use.input.clone())
            .map_err(|e| ActionError::ParseError(format!("Invalid tool input: {}", e)))?;

        self.state
            .set_last_action(serde_json::to_string(&action).unwrap_or_default())
            .await;
        self.emit_state_update().await;

        // Execute the action
        let mut result = execute_action(&action, confirm_dangerous)?;

        // Attach the tool_use_id to the result
        result = result.with_tool_use_id(tool_use.id.clone());

        Ok(result)
    }

    async fn emit_state_update(&self) {
        let state = self.state.get_state().await;
        let _ = self.app_handle.emit("agent-state", state);
    }
}
