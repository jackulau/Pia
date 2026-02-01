use crate::agent::conversation::{ConversationHistory, Message};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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

/// A tool definition for Claude's native tool_use protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// A tool_use response from Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// Response from an LLM provider - can be either a tool use or raw text
#[derive(Debug, Clone)]
pub enum LlmResponse {
    /// Native tool use response (from Anthropic)
    ToolUse(ToolUse),
    /// Raw text response (fallback for JSON parsing)
    Text(String),
}

impl LlmResponse {
    /// Convert to a string representation for logging/conversation history
    pub fn to_string_repr(&self) -> String {
        match self {
            LlmResponse::ToolUse(tool_use) => {
                serde_json::to_string(tool_use).unwrap_or_else(|_| format!("{:?}", tool_use))
            }
            LlmResponse::Text(text) => text.clone(),
        }
    }
}

/// Build tool definitions for all computer use actions
pub fn build_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "click".to_string(),
            description: "Click at coordinates on screen".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "integer",
                        "description": "X coordinate to click"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate to click"
                    },
                    "button": {
                        "type": "string",
                        "enum": ["left", "right", "middle"],
                        "default": "left",
                        "description": "Mouse button to click"
                    }
                },
                "required": ["x", "y"]
            }),
        },
        Tool {
            name: "double_click".to_string(),
            description: "Double click at coordinates on screen".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "integer",
                        "description": "X coordinate to double-click"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate to double-click"
                    }
                },
                "required": ["x", "y"]
            }),
        },
        Tool {
            name: "move".to_string(),
            description: "Move mouse to coordinates without clicking".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "integer",
                        "description": "X coordinate to move to"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate to move to"
                    }
                },
                "required": ["x", "y"]
            }),
        },
        Tool {
            name: "type".to_string(),
            description: "Type text using the keyboard".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to type"
                    }
                },
                "required": ["text"]
            }),
        },
        Tool {
            name: "key".to_string(),
            description: "Press a key with optional modifiers".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "key": {
                        "type": "string",
                        "description": "Key to press (e.g., 'enter', 'tab', 'a', 'escape')"
                    },
                    "modifiers": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["ctrl", "alt", "shift", "meta"]
                        },
                        "default": [],
                        "description": "Modifier keys to hold (meta is cmd on macOS)"
                    }
                },
                "required": ["key"]
            }),
        },
        Tool {
            name: "scroll".to_string(),
            description: "Scroll at a position on screen".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "integer",
                        "description": "X coordinate to scroll at"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate to scroll at"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down", "left", "right"],
                        "description": "Direction to scroll"
                    },
                    "amount": {
                        "type": "integer",
                        "default": 3,
                        "description": "Number of scroll increments"
                    }
                },
                "required": ["x", "y", "direction"]
            }),
        },
        Tool {
            name: "complete".to_string(),
            description: "Mark the task as completed successfully".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Completion message describing what was accomplished"
                    }
                },
                "required": ["message"]
            }),
        },
        Tool {
            name: "error".to_string(),
            description: "Report an error or inability to proceed".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Error message describing the issue"
                    }
                },
                "required": ["message"]
            }),
        },
    ]
}

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
    ) -> Result<(LlmResponse, TokenMetrics), LlmError>;

    /// Legacy method for sending a single image with instruction.
    /// Kept for backwards compatibility but delegates to send_with_history.
    async fn send_with_image(
        &self,
        instruction: &str,
        image_base64: &str,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(LlmResponse, TokenMetrics), LlmError> {
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

/// Build system prompt for tool-based providers (simplified, tools are defined via API)
pub fn build_system_prompt_for_tools(screen_width: u32, screen_height: u32) -> String {
    format!(
        r#"You are a computer use agent. You can see the user's screen and control their mouse and keyboard to complete tasks.

Screen dimensions: {screen_width}x{screen_height} pixels

Guidelines:
- Analyze the screenshot carefully before acting
- Use coordinates that match visible UI elements
- Be precise with click locations
- Wait for UI to update between actions (the system handles this)
- Use the "complete" tool when the task is done
- Use the "error" tool if you cannot proceed

Use one of the provided tools to perform your next action."#
    )
}

/// Build system prompt for JSON-based providers (includes action definitions in prompt)
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

8. Triple click (select entire line):
   {{"action": "triple_click", "x": 100, "y": 200}}
   Useful for selecting entire lines of text

9. Right click (context menu):
   {{"action": "right_click", "x": 100, "y": 200}}

10. Wait/pause execution:
    {{"action": "wait", "duration_ms": 1000}}
    Useful when waiting for UI elements to load or animations to complete

11. Complete the task:
    {{"action": "complete", "message": "Task completed successfully"}}

12. Report an error or inability to proceed:
    {{"action": "error", "message": "Cannot find the required element"}}

13. Execute multiple actions in sequence (batch):
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

Note: Actions are automatically retried up to 3 times if they fail or have no visible effect.
If an action consistently fails, try:
- Adjusting coordinates slightly (elements may have shifted)
- Using a different approach (e.g., keyboard navigation instead of clicking)
- Waiting longer for elements to load by trying again

Respond with ONLY the JSON action, no other text."#
    )
}
