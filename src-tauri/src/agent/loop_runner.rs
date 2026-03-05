use super::action::{execute_action, execute_action_with_delay, execute_action_with_retry, parse_llm_response, Action, ActionError, ActionResult};
use super::conversation::ConversationHistory;
use super::delay::DelayController;
use super::history::{ActionEntry, ActionHistory, ActionRecord};
use super::queue::{QueueFailureMode, QueueManager};
use super::recovery::{
    classify_capture_error, classify_llm_error, retry_with_policy, ErrorClassification,
    RetryPolicy,
};
use super::state::{AgentStateManager, AgentStatus, ConfirmationResponse, ExecutionMode};
use crate::capture::{capture_primary_screen, CaptureError, Screenshot};
use crate::config::Config;
use crate::llm::{
    AnthropicProvider, GlmProvider, LlmProvider, OllamaProvider, OpenAICompatibleProvider,
    OpenAIProvider, OpenRouterProvider, ToolResult,
};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout, Duration, Instant};

/// Minimum interval between state emissions to avoid flooding the frontend
const STATE_EMISSION_MIN_INTERVAL_MS: u64 = 50;

#[derive(Clone, Serialize)]
struct HistoryEvent {
    instruction: String,
    success: bool,
}

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
    #[error("Action denied by user")]
    ActionDenied,
    #[error("Too many consecutive errors: {0}")]
    TooManyErrors(u32),
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

const MAX_CONSECUTIVE_ERRORS: u32 = 3;

pub struct AgentLoop {
    state: AgentStateManager,
    config: Config,
    app_handle: AppHandle,
    queue: Option<QueueManager>,
    preview_mode: bool,
    action_history: Arc<RwLock<ActionHistory>>,
    /// Tracks last state emission time for debouncing
    last_emission: std::sync::Mutex<Instant>,
}

impl AgentLoop {
    pub fn new(
        state: AgentStateManager,
        config: Config,
        app_handle: AppHandle,
        action_history: Arc<RwLock<ActionHistory>>,
    ) -> Self {
        let preview_mode = config.general.preview_mode;
        Self {
            state,
            config,
            app_handle,
            queue: None,
            preview_mode,
            action_history,
            last_emission: std::sync::Mutex::new(Instant::now()),
        }
    }

    pub fn with_queue(mut self, queue: QueueManager) -> Self {
        self.queue = Some(queue);
        self
    }

    fn create_provider(&self) -> Result<Box<dyn LlmProvider>, LoopError> {
        let provider_name = &self.config.general.default_provider;
        let connect_timeout = Duration::from_secs(self.config.general.connect_timeout_secs);
        let response_timeout = Duration::from_secs(self.config.general.response_timeout_secs);

        match provider_name.as_str() {
            "ollama" => {
                let config = self
                    .config
                    .providers
                    .ollama
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(OllamaProvider::with_timeouts(
                    config.host.clone(),
                    config.model.clone(),
                    config.temperature,
                    connect_timeout,
                    response_timeout,
                )))
            }
            "anthropic" => {
                let config = self
                    .config
                    .providers
                    .anthropic
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(AnthropicProvider::with_timeouts(
                    config.api_key.clone(),
                    config.model.clone(),
                    config.temperature,
                    connect_timeout,
                    response_timeout,
                )))
            }
            "openai" => {
                let config = self
                    .config
                    .providers
                    .openai
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(OpenAIProvider::with_timeouts(
                    config.api_key.clone(),
                    config.model.clone(),
                    config.temperature,
                    connect_timeout,
                    response_timeout,
                )))
            }
            "openrouter" => {
                let config = self
                    .config
                    .providers
                    .openrouter
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(OpenRouterProvider::with_timeouts(
                    config.api_key.clone(),
                    config.model.clone(),
                    config.temperature,
                    connect_timeout,
                    response_timeout,
                )))
            }
            "glm" => {
                let config = self
                    .config
                    .providers
                    .glm
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(GlmProvider::new(
                    config.api_key.clone(),
                    config.model.clone(),
                    config.temperature,
                )))
            }
            "openai-compatible" => {
                let config = self
                    .config
                    .providers
                    .openai_compatible
                    .as_ref()
                    .ok_or(LoopError::NoProvider)?;
                Ok(Box::new(OpenAICompatibleProvider::new(
                    config.base_url.clone(),
                    config.api_key.clone(),
                    config.model.clone(),
                    config.temperature,
                )))
            }
            _ => Err(LoopError::NoProvider),
        }
    }

    pub async fn run(&self, instruction: String) -> Result<(), LoopError> {
        self.run_with_mode(instruction, ExecutionMode::Normal).await
    }

    pub async fn run_recording(&self, instruction: String) -> Result<(), LoopError> {
        self.run_with_mode(instruction, ExecutionMode::Recording).await
    }

    async fn run_with_mode(&self, instruction: String, mode: ExecutionMode) -> Result<(), LoopError> {
        let provider = self.create_provider()?;
        let max_iterations = self.config.general.max_iterations;
        let confirm_dangerous = self.config.general.confirm_dangerous_actions;
        let show_overlay = self.config.general.show_coordinate_overlay;

        // Initialize conversation history for this task
        let mut conversation = ConversationHistory::new();
        conversation.set_original_instruction(instruction.clone());

        // Clear action history for new session
        {
            let mut history = self.action_history.write().await;
            history.clear();
        }

        self.state.start_with_mode(instruction.clone(), max_iterations, mode).await;
        self.state.update_undo_state(false, None).await;
        self.emit_state_update_immediate().await;

        let result = self.run_loop(&*provider, &instruction, max_iterations, confirm_dangerous, show_overlay, &mut conversation).await;

        // Complete the history session with final status
        let status = match &result {
            Ok(_) => "completed",
            Err(LoopError::Stopped) => "stopped",
            Err(LoopError::MaxIterations) => "max_iterations",
            Err(_) => "error",
        };
        self.state.history().complete_session(status).await;

        result
    }

    async fn run_loop(
        &self,
        provider: &dyn LlmProvider,
        instruction: &str,
        max_iterations: u32,
        confirm_dangerous: bool,
        show_overlay: bool,
        conversation: &mut ConversationHistory,
    ) -> Result<(), LoopError> {
        // Initialize delay controller and target iteration delay
        let speed_multiplier = self.config.general.speed_multiplier;
        let delay_controller = DelayController::new(speed_multiplier);
        let target_iteration_delay = delay_controller.iteration_delay();

        loop {
            // Check if should stop
            if self.state.should_stop() {
                self.state.set_status(AgentStatus::Idle).await;
                self.emit_state_update_immediate().await;
                return Err(LoopError::Stopped);
            }

            // Check consecutive error limit
            let consecutive_errors = self.state.get_consecutive_errors();
            if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                self.state
                    .set_error(format!(
                        "Too many consecutive errors ({})",
                        consecutive_errors
                    ))
                    .await;
                self.emit_state_update_immediate().await;
                return Err(LoopError::TooManyErrors(consecutive_errors));
            }

            // Check if should pause
            if self.state.should_pause() {
                self.state.set_status(AgentStatus::Paused).await;
                self.emit_state_update_immediate().await;

                // Wait while paused
                while self.state.should_pause() && !self.state.should_stop() {
                    sleep(Duration::from_millis(100)).await;
                }

                // Check if stopped while paused
                if self.state.should_stop() {
                    self.state.set_status(AgentStatus::Idle).await;
                    self.emit_state_update_immediate().await;
                    return Err(LoopError::Stopped);
                }

                // Resume running
                self.state.set_status(AgentStatus::Running).await;
                self.emit_state_update_immediate().await;
            }

            // Check iteration limit (now uses atomic, no await needed)
            let iteration = self.state.increment_iteration();
            if iteration > max_iterations {
                self.state
                    .set_error("Max iterations reached".to_string())
                    .await;
                self.emit_state_update_immediate().await;
                return Err(LoopError::MaxIterations);
            }

            // Capture screenshot with retry
            let screenshot = match self.capture_with_retry().await {
                Ok(s) => s,
                Err(e) => {
                    self.state.increment_consecutive_errors();
                    self.state.set_error(e.to_string()).await;
                    self.emit_state_update_immediate().await;
                    return Err(e.into());
                }
            };

            // Add user message with current screenshot to conversation
            conversation.add_user_message(
                &instruction,
                Some(screenshot.base64.clone()),
                Some(screenshot.width),
                Some(screenshot.height),
            );

            // Store screenshot in state for frontend preview
            self.state
                .set_last_screenshot(screenshot.base64.clone())
                .await;
            self.emit_state_update().await;

            // Create callback for chunk streaming
            let app_handle = self.app_handle.clone();
            let on_chunk: Box<dyn Fn(&str) + Send + Sync> = Box::new(move |chunk: &str| {
                let _ = app_handle.emit("llm-chunk", chunk.to_string());
            });

            // Track LLM response time for adaptive delay
            let llm_start = Instant::now();

            // Send conversation history to LLM with retry logic
            let llm_result = provider
                .send_with_history(
                    &conversation,
                    screenshot.width,
                    screenshot.height,
                    on_chunk,
                )
                .await;

            let (response, metrics) = match llm_result {
                Ok((resp, met)) => {
                    // Reset consecutive errors on success
                    self.state.reset_consecutive_errors();
                    (resp, met)
                }
                Err(e) => {
                    self.state.increment_consecutive_errors();
                    self.state.set_error(e.to_string()).await;
                    self.emit_state_update_immediate().await;

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

            // Add assistant response to conversation
            let response_str = response.to_string_repr();
            conversation.add_assistant_message(&response_str);

            let llm_elapsed = llm_start.elapsed();

            // Update metrics atomically (no await needed)
            self.state.update_metrics(
                metrics.tokens_per_second(),
                metrics.input_tokens,
                metrics.output_tokens,
            );
            self.state.history().update_metrics(metrics.input_tokens, metrics.output_tokens).await;

            // Parse action - on parse error, send feedback to LLM
            let action = match parse_llm_response(&response) {
                Ok(a) => a,
                Err(parse_err) => {
                    self.state.increment_consecutive_errors();

                    // Emit parse error feedback
                    let _ = self.app_handle.emit(
                        "parse-error",
                        format!("Failed to parse LLM response: {}", parse_err),
                    );

                    // Continue to next iteration - the LLM will see the error
                    // in subsequent iterations via conversation context
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };

            // Serialize action once - reuse for display, history, and details
            let action_value = serde_json::to_value(&action).unwrap_or_default();
            let action_str = action_value.to_string();
            let action_display = if self.preview_mode {
                format!("[PREVIEW] {}", action_str)
            } else {
                action_str.clone()
            };
            self.state.set_last_action(action_display).await;

            // Emit state once after LLM + action parsing (batched update)
            self.emit_state_update().await;

            // Skip execution in preview mode
            if self.preview_mode {
                // In preview mode, check if action would be a completion
                if let Action::Complete { .. } = action {
                    self.state.complete(Some("Preview completed".to_string())).await;
                    self.emit_state_update().await;
                    return Ok(());
                }
                // Continue to next iteration without executing
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            // Emit coordinate to overlay if enabled
            if show_overlay {
                self.emit_coordinate(&action);
            }

            // Emit visual feedback indicator
            self.emit_action_indicator(&action);

            // Show cursor indicator before executing action
            self.show_cursor_indicator(&action).await;

            // Brief pause to let user see the indicator
            sleep(Duration::from_millis(300)).await;

            // Prepare action details for history logging (reuse serialized value)
            let action_type = Self::get_action_type(&action);

            match execute_action_with_delay(&action, confirm_dangerous, delay_controller.click_delay()).await {
                Ok(result) => {
                    // Add successful tool result to conversation
                    conversation.add_tool_result(true, result.message.clone(), None);


                    // Record successful action to history
                    let entry = ActionEntry {
                        timestamp: Utc::now(),
                        iteration,
                        action_type: action_type.clone(),
                        action_details: action_value.clone(),
                        screenshot_base64: Some(screenshot.base64.clone()),
                        llm_response: response_str.clone(),
                        success: true,
                        error_message: None,
                        result_message: result.message.clone(),
                    };
                    self.state.history().add_entry(entry).await;

                    // Reset consecutive errors on successful action
                    self.state.reset_consecutive_errors();

                    // Hide cursor indicator after action
                    self.hide_cursor_indicator().await;

                    // Update retry statistics if any retries occurred
                    if result.retry_count > 0 {
                        self.state.update_retry_stats(result.retry_count).await;
                        log::info!("Action succeeded after {} retries", result.retry_count);
                    }

                    // Record successful action in history (unless it's a terminal action)
                    if !result.completed {
                        let record = ActionRecord::new(action.clone(), true);
                        let mut history = self.action_history.write().await;
                        history.push(record);

                        // Update undo state
                        let can_undo = history.can_undo();
                        let last_undoable = history.get_last_undoable_description();
                        drop(history);
                        self.state.update_undo_state(can_undo, last_undoable).await;
                        self.emit_state_update().await;
                    }


                    if result.completed {
                        self.state.complete(result.message).await;
                        self.emit_state_update_immediate().await;
                        // Emit history event for successful completion
                        let _ = self.app_handle.emit(
                            "instruction-completed",
                            HistoryEvent {
                                instruction: instruction.to_string(),
                                success: true,
                            },
                        );
                        return Ok(());
                    }
                }
                Err(ActionError::RequiresConfirmation(msg)) => {
                    // Add pending confirmation to conversation
                    conversation.add_tool_result(
                        false,
                        None,
                        Some(format!("Action requires confirmation: {}", msg)),
                    );

                    // Record confirmation-required action to history
                    let entry = ActionEntry {
                        timestamp: Utc::now(),
                        iteration,
                        action_type: action_type.clone(),
                        action_details: action_value.clone(),
                        screenshot_base64: Some(screenshot.base64.clone()),
                        llm_response: response_str.clone(),
                        success: false,
                        error_message: Some(format!("Requires confirmation: {}", msg)),
                        result_message: None,
                    };
                    self.state.history().add_entry(entry).await;

                    // Reset confirmation channel for fresh state
                    self.state.reset_confirmation_channel().await;

                    // Set pending action and status
                    self.state.set_pending_action(Some(msg.clone())).await;
                    self.state.set_status(AgentStatus::AwaitingConfirmation).await;
                    self.emit_state_update_immediate().await;

                    // Emit confirmation request to frontend
                    let _ = self.app_handle.emit("confirmation-required", msg);

                    // Wait for user response with 30 second timeout
                    let confirmation_timeout = Duration::from_secs(30);
                    let response = timeout(confirmation_timeout, self.state.await_confirmation()).await;

                    // Clear pending action
                    self.state.set_pending_action(None).await;

                    match response {
                        Ok(Some(ConfirmationResponse::Confirmed)) => {
                            // User confirmed, continue execution
                            self.state.set_status(AgentStatus::Running).await;
                            self.emit_state_update_immediate().await;
                        }
                        Ok(Some(ConfirmationResponse::Denied)) | Ok(None) | Err(_) => {
                            // User denied, no response, or timeout - abort
                            self.state.set_status(AgentStatus::Idle).await;
                            self.state.set_error("Action denied or timed out".to_string()).await;
                            self.emit_state_update_immediate().await;
                            return Err(LoopError::ActionDenied);
                        }
                    }

                    // Hide cursor indicator after confirmation period
                    self.hide_cursor_indicator().await;

                    if self.state.should_stop() {
                        self.state.set_status(AgentStatus::Idle).await;
                        self.emit_state_update_immediate().await;
                        return Err(LoopError::Stopped);
                    }
                }
                Err(e) => {
                    // Add error to conversation before returning
                    conversation.add_tool_result(false, None, Some(e.to_string()));

                    // Record failed action to history
                    let entry = ActionEntry {
                        timestamp: Utc::now(),
                        iteration,
                        action_type: action_type.clone(),
                        action_details: action_value.clone(),
                        screenshot_base64: Some(screenshot.base64.clone()),
                        llm_response: response_str.clone(),
                        success: false,
                        error_message: Some(e.to_string()),
                        result_message: None,
                    };
                    self.state.history().add_entry(entry).await;

                    // Increment consecutive errors
                    self.state.increment_consecutive_errors();

                    // Hide cursor indicator on error
                    self.hide_cursor_indicator().await;

                    // Record failed action in undo history
                    let record = ActionRecord::new(action.clone(), false);
                    let mut history = self.action_history.write().await;
                    history.push(record);
                    drop(history);


                    self.state.set_error(e.to_string()).await;
                    self.emit_state_update_immediate().await;

                    // Emit history event for failed completion
                    let _ = self.app_handle.emit(
                        "instruction-completed",
                        HistoryEvent {
                            instruction: instruction.to_string(),
                            success: false,
                        },
                    );

                    // Action errors are generally not retryable
                    return Err(e.into());
                }
            }

            // Adaptive delay: if LLM took >= target, skip delay; otherwise sleep remaining
            if llm_elapsed < target_iteration_delay {
                let remaining = target_iteration_delay - llm_elapsed;
                sleep(remaining).await;
            }
            // If LLM took longer than target, no delay needed
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

    fn get_action_type(action: &Action) -> String {
        match action {
            Action::Click { .. } => "click".to_string(),
            Action::DoubleClick { .. } => "double_click".to_string(),
            Action::Move { .. } => "move".to_string(),
            Action::Type { .. } => "type".to_string(),
            Action::Key { .. } => "key".to_string(),
            Action::Scroll { .. } => "scroll".to_string(),
            Action::Drag { .. } => "drag".to_string(),
            Action::TripleClick { .. } => "triple_click".to_string(),
            Action::RightClick { .. } => "right_click".to_string(),
            Action::Wait { .. } => "wait".to_string(),
            Action::Complete { .. } => "complete".to_string(),
            Action::Error { .. } => "error".to_string(),
            Action::Batch { .. } => "batch".to_string(),
            Action::WaitForElement { .. } => "wait_for_element".to_string(),
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
                self.emit_state_update_immediate().await;
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
                    self.emit_state_update_immediate().await;
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
                    current_instruction: instruction.to_string(),
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
                            self.emit_state_update_immediate().await;
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
                self.emit_state_update_immediate().await;
                self.emit_queue_update().await;
                return Ok(());
            }

            // Delay between queue items
            if queue_delay.as_millis() > 0 {
                sleep(queue_delay).await;
            }
        }
    }

    /// Emit state update with debouncing (min 50ms between emissions).
    /// Use `emit_state_update_immediate` for status transitions that must be seen immediately.
    async fn emit_state_update(&self) {
        let min_interval = Duration::from_millis(STATE_EMISSION_MIN_INTERVAL_MS);
        let should_emit = {
            let mut last = self.last_emission.lock().unwrap();
            let now = Instant::now();
            if now.duration_since(*last) >= min_interval {
                *last = now;
                true
            } else {
                false
            }
        };
        if should_emit {
            let state = self.state.get_state().await;
            let _ = self.app_handle.emit("agent-state", state);
        }
    }

    /// Emit state update immediately, bypassing debounce.
    /// Used for status transitions (idle, completed, error, paused) that the frontend must see.
    async fn emit_state_update_immediate(&self) {
        {
            let mut last = self.last_emission.lock().unwrap();
            *last = Instant::now();
        }
        let state = self.state.get_state().await;
        let _ = self.app_handle.emit("agent-state", state);
    }

    fn emit_coordinate(&self, action: &Action) {
        let (x, y, action_type) = match action {
            Action::Click { x, y, .. } => (*x, *y, "click"),
            Action::DoubleClick { x, y } => (*x, *y, "double_click"),
            Action::Move { x, y } => (*x, *y, "move"),
            Action::Scroll { x, y, .. } => (*x, *y, "scroll"),
            _ => return,
        };

        let _ = self.app_handle.emit(
            "show-coordinate",
            json!({
                "x": x,
                "y": y,
                "action_type": action_type
            }),
        );
    }

    fn emit_action_indicator(&self, action: &Action) {
        let payload = match action {
            Action::Click { x, y, button } => serde_json::json!({
                "action": button.to_lowercase(),
                "x": x,
                "y": y,
                "label": format!("{} click", button)
            }),
            Action::DoubleClick { x, y } => serde_json::json!({
                "action": "double_click",
                "x": x,
                "y": y,
                "label": "double click"
            }),
            Action::Move { x, y } => serde_json::json!({
                "action": "move",
                "x": x,
                "y": y,
                "label": "move"
            }),
            Action::Type { text } => serde_json::json!({
                "action": "type",
                "text": text
            }),
            Action::Key { key, modifiers } => serde_json::json!({
                "action": "key",
                "key": key,
                "modifiers": modifiers
            }),
            Action::Scroll { x, y, direction, amount: _ } => serde_json::json!({
                "action": "scroll",
                "x": x,
                "y": y,
                "direction": direction,
                "label": format!("scroll {}", direction)
            }),
            Action::Drag { start_x, start_y, end_x, end_y, .. } => serde_json::json!({
                "action": "drag",
                "start_x": start_x,
                "start_y": start_y,
                "end_x": end_x,
                "end_y": end_y,
                "label": "drag"
            }),
            Action::TripleClick { x, y } => serde_json::json!({
                "action": "triple_click",
                "x": x,
                "y": y,
                "label": "triple click"
            }),
            Action::RightClick { x, y } => serde_json::json!({
                "action": "right_click",
                "x": x,
                "y": y,
                "label": "right click"
            }),
            Action::Wait { duration_ms } => serde_json::json!({
                "action": "wait",
                "duration_ms": duration_ms,
                "label": "wait"
            }),
            Action::WaitForElement { timeout_ms, description } => serde_json::json!({
                "action": "wait_for_element",
                "timeout_ms": timeout_ms,
                "description": description,
                "label": format!("wait for: {}", description)
            }),
            Action::Batch { actions } => serde_json::json!({
                "action": "batch",
                "count": actions.len(),
                "label": format!("batch ({} actions)", actions.len())
            }),
            Action::Complete { .. } | Action::Error { .. } => return,
        };

        let _ = self.app_handle.emit("show-action-indicator", payload);
    }

    async fn show_cursor_indicator(&self, action: &Action) {
        let (x, y, action_type) = match action {
            Action::Click { x, y, .. } => (*x, *y, "click"),
            Action::DoubleClick { x, y } => (*x, *y, "double_click"),
            Action::Move { x, y } => (*x, *y, "move"),
            Action::Scroll { x, y, .. } => (*x, *y, "scroll"),
            _ => return,
        };

        if let Some(overlay) = self.app_handle.get_webview_window("cursor-overlay") {
            // Get available monitors
            if let Ok(monitors) = overlay.available_monitors() {
                // Find the monitor containing the point
                let target_monitor = monitors
                    .iter()
                    .find(|m| {
                        let pos = m.position();
                        let size = m.size();
                        x >= pos.x && x < pos.x + size.width as i32 &&
                        y >= pos.y && y < pos.y + size.height as i32
                    })
                    .or_else(|| monitors.first());

                if let Some(monitor) = target_monitor {
                    let monitor_pos = monitor.position();
                    let monitor_size = monitor.size();

                    // Position overlay to cover the monitor
                    let _ = overlay.set_position(PhysicalPosition::new(monitor_pos.x, monitor_pos.y));
                    let _ = overlay.set_size(PhysicalSize::new(monitor_size.width, monitor_size.height));

                    // Calculate relative position
                    let relative_x = x - monitor_pos.x;
                    let relative_y = y - monitor_pos.y;

                    // Show overlay and emit position
                    let _ = overlay.show();

                    #[derive(Clone, Serialize)]
                    struct CursorPayload {
                        x: i32,
                        y: i32,
                        action_type: String,
                    }

                    let _ = overlay.emit("show-cursor-indicator", CursorPayload {
                        x: relative_x,
                        y: relative_y,
                        action_type: action_type.to_string(),
                    });
                }
            }
        }
    }

    async fn hide_cursor_indicator(&self) {
        if let Some(overlay) = self.app_handle.get_webview_window("cursor-overlay") {
            let _ = overlay.emit("hide-cursor-indicator", ());
            // Wait for animation
            sleep(Duration::from_millis(150)).await;
            let _ = overlay.hide();
        }
    }

    async fn emit_queue_update(&self) {
        if let Some(queue) = &self.queue {
            let queue_state = queue.get_state().await;
            let _ = self.app_handle.emit("queue-update", queue_state);
        }
    }
}
