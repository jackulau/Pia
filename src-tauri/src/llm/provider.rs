use crate::agent::conversation::{ConversationHistory, Message};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Stream error: {0}")]
    StreamError(String),
    #[error("Provider not configured")]
    NotConfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_duration: Duration,
}

impl TokenMetrics {
    pub fn tokens_per_second(&self) -> f64 {
        if self.total_duration.as_secs_f64() > 0.0 {
            self.output_tokens as f64 / self.total_duration.as_secs_f64()
        } else {
            0.0
        }
    }
}

pub type ChunkCallback = Box<dyn Fn(&str) + Send + Sync>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Sends a message with conversation history to the LLM.
    /// This is the preferred method that includes context from previous iterations.
    async fn send_with_history(
        &self,
        history: &ConversationHistory,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(String, TokenMetrics), LlmError>;

    /// Legacy method for sending a single image with instruction.
    /// Kept for backwards compatibility but delegates to send_with_history.
    async fn send_with_image(
        &self,
        instruction: &str,
        image_base64: &str,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(String, TokenMetrics), LlmError> {
        // Create a temporary conversation history with just this message
        let mut history = ConversationHistory::new();
        history.add_user_message(
            instruction,
            Some(image_base64.to_string()),
            Some(screen_width),
            Some(screen_height),
        );
        self.send_with_history(&history, screen_width, screen_height, on_chunk)
            .await
    }

    fn name(&self) -> &str;
}

/// Helper to convert conversation history to provider-specific message format.
/// Returns a Vec of tuples: (role, text_content, optional_image_base64)
pub fn history_to_messages(history: &ConversationHistory) -> Vec<(String, String, Option<String>)> {
    history
        .get_messages()
        .iter()
        .map(|msg| match msg {
            Message::User {
                instruction,
                screenshot_base64,
                ..
            } => (
                "user".to_string(),
                instruction.clone(),
                screenshot_base64.clone(),
            ),
            Message::Assistant { content } => ("assistant".to_string(), content.clone(), None),
            Message::ToolResult {
                success,
                message,
                error,
            } => {
                let text = if *success {
                    format!(
                        "Action executed successfully. {}",
                        message.as_deref().unwrap_or("")
                    )
                } else {
                    format!(
                        "Action failed. {}",
                        error.as_deref().unwrap_or("Unknown error")
                    )
                };
                ("user".to_string(), text, None)
            }
        })
        .collect()
}

pub fn build_system_prompt(screen_width: u32, screen_height: u32) -> String {
    format!(
        r#"You are a computer use agent. You can see the user's screen and control their mouse and keyboard to complete tasks.

Screen dimensions: {screen_width}x{screen_height} pixels

You must respond with a single JSON action. Available actions:

1. Click at coordinates:
   {{"action": "click", "x": 100, "y": 200, "button": "left"}}
   button can be "left", "right", or "middle"

2. Double click:
   {{"action": "double_click", "x": 100, "y": 200}}

3. Type text:
   {{"action": "type", "text": "Hello World"}}

4. Press a key with optional modifiers:
   {{"action": "key", "key": "enter"}}
   {{"action": "key", "key": "c", "modifiers": ["ctrl"]}}
   {{"action": "key", "key": "v", "modifiers": ["ctrl"]}}
   Available modifiers: "ctrl", "alt", "shift", "meta" (cmd on macOS)

5. Scroll at position:
   {{"action": "scroll", "x": 500, "y": 300, "direction": "down", "amount": 3}}
   direction can be "up", "down", "left", or "right"

6. Move mouse (without clicking):
   {{"action": "move", "x": 100, "y": 200}}

7. Drag from one point to another:
   {{"action": "drag", "start_x": 100, "start_y": 200, "end_x": 300, "end_y": 200}}
   Click and drag from start position to end position.
   Optional: "button" (default "left"), "duration_ms" (default 500, max 5000)
   Use for: moving files, resizing windows, adjusting sliders, selecting text

8. Complete the task:
   {{"action": "complete", "message": "Task completed successfully"}}

9. Report an error or inability to proceed:
   {{"action": "error", "message": "Cannot find the required element"}}

9. Execute multiple actions in sequence (batch):
   {{"action": "batch", "actions": [{{"action": "type", "text": "hello"}}, {{"action": "key", "key": "tab"}}]}}
   Use for predictable action sequences that don't need intermediate screenshots.
   Max 10 actions per batch. Batch stops on first failure or complete action.

Guidelines:
- Analyze the screenshot carefully before acting
- Use coordinates that match visible UI elements
- Be precise with click locations
- Wait for UI to update between actions (the system handles this)
- Use "complete" when the task is done
- Use "error" if you cannot proceed

Respond with ONLY the JSON action, no other text."#
    )
}
