use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

/// Represents a tool result to be sent back to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The ID of the tool_use block this result corresponds to
    pub tool_use_id: String,
    /// Whether this result represents an error
    pub is_error: bool,
    /// The content/output of the tool execution
    pub content: String,
}

impl ToolResult {
    /// Create a successful tool result
    pub fn success(tool_use_id: String, content: String) -> Self {
        Self {
            tool_use_id,
            is_error: false,
            content,
        }
    }

    /// Create an error tool result
    pub fn error(tool_use_id: String, error_message: String) -> Self {
        Self {
            tool_use_id,
            is_error: true,
            content: error_message,
        }
    }

    /// Convert to JSON format for API
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "tool_result",
            "tool_use_id": self.tool_use_id,
            "is_error": self.is_error,
            "content": self.content,
        })
    }
}

/// Represents a tool use request from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Unique identifier for this tool use
    pub id: String,
    /// Name of the tool being called
    pub name: String,
    /// Input parameters for the tool (as JSON value)
    pub input: serde_json::Value,
}

/// Response from the LLM that may contain text and/or tool use requests
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Text content from the response (if any)
    pub text: Option<String>,
    /// Tool use requests from the response (if any)
    pub tool_uses: Vec<ToolUse>,
    /// Token metrics for this response
    pub metrics: TokenMetrics,
    /// The stop reason for this response
    pub stop_reason: Option<String>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a message with an image and get a response (legacy text-based)
    async fn send_with_image(
        &self,
        instruction: &str,
        image_base64: &str,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(String, TokenMetrics), LlmError>;

    /// Send a message with tools enabled and get a structured response
    /// Returns the response with potential tool_use blocks
    async fn send_with_tools(
        &self,
        instruction: &str,
        image_base64: &str,
        screen_width: u32,
        screen_height: u32,
        tool_results: Option<Vec<ToolResult>>,
        on_chunk: ChunkCallback,
    ) -> Result<LlmResponse, LlmError>;

    /// Check if this provider supports native tool use
    fn supports_tools(&self) -> bool {
        false
    }

    fn name(&self) -> &str;
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

7. Complete the task:
   {{"action": "complete", "message": "Task completed successfully"}}

8. Report an error or inability to proceed:
   {{"action": "error", "message": "Cannot find the required element"}}

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
