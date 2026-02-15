use crate::agent::conversation::{ConversationHistory, Message};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
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
    /// Native tool use response (from Anthropic), with optional reasoning text
    ToolUse { tool_use: ToolUse, reasoning: Option<String> },
    /// Raw text response (fallback for JSON parsing)
    Text(String),
}

impl LlmResponse {
    /// Convert to a string representation for logging/conversation history
    pub fn to_string_repr(&self) -> String {
        match self {
            LlmResponse::ToolUse { tool_use, reasoning } => {
                let tool_json = serde_json::to_string(tool_use).unwrap_or_else(|_| format!("{:?}", tool_use));
                if let Some(r) = reasoning {
                    format!("{}\n{}", r, tool_json)
                } else {
                    tool_json
                }
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
            name: "drag".to_string(),
            description: "Click and drag from one position to another. Use for moving files, resizing windows, adjusting sliders, or selecting text.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "start_x": { "type": "integer", "description": "X coordinate of drag start" },
                    "start_y": { "type": "integer", "description": "Y coordinate of drag start" },
                    "end_x": { "type": "integer", "description": "X coordinate of drag end" },
                    "end_y": { "type": "integer", "description": "Y coordinate of drag end" },
                    "button": { "type": "string", "enum": ["left", "right", "middle"], "default": "left", "description": "Mouse button" },
                    "duration_ms": { "type": "integer", "default": 500, "description": "Drag duration in ms (max 5000)" }
                },
                "required": ["start_x", "start_y", "end_x", "end_y"]
            }),
        },
        Tool {
            name: "triple_click".to_string(),
            description: "Triple click at coordinates to select an entire line of text".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "X coordinate" },
                    "y": { "type": "integer", "description": "Y coordinate" }
                },
                "required": ["x", "y"]
            }),
        },
        Tool {
            name: "right_click".to_string(),
            description: "Right click at coordinates to open a context menu".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "X coordinate" },
                    "y": { "type": "integer", "description": "Y coordinate" }
                },
                "required": ["x", "y"]
            }),
        },
        Tool {
            name: "wait".to_string(),
            description: "Wait/pause execution. Useful when waiting for UI to load or animations to complete.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "duration_ms": { "type": "integer", "default": 1000, "description": "Duration to wait in milliseconds" }
                },
                "required": []
            }),
        },
        Tool {
            name: "wait_for_element".to_string(),
            description: "Wait for a UI element or screen change before proceeding. Use after clicking buttons that trigger loading or navigating to new pages.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": { "type": "string", "description": "What to wait for (e.g., 'page to load')" },
                    "timeout_ms": { "type": "integer", "default": 5000, "description": "Max wait time in ms (max 10000)" }
                },
                "required": ["description"]
            }),
        },
        Tool {
            name: "batch".to_string(),
            description: "Execute multiple actions in sequence without intermediate screenshots. Max 10 actions. Stops on first failure.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "actions": {
                        "type": "array",
                        "description": "Array of action objects to execute in sequence",
                        "items": { "type": "object" },
                        "maxItems": 10
                    }
                },
                "required": ["actions"]
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
            Some(Arc::new(image_base64.to_string())),
            Some(screen_width),
            Some(screen_height),
        );
        self.send_with_history(&history, screen_width, screen_height, on_chunk)
            .await
    }

    /// Send a message with tools enabled and get a structured response
    /// Returns the response with potential tool_use blocks.
    /// Default implementation returns an error since most providers don't support native tools.
    async fn send_with_tools(
        &self,
        _instruction: &str,
        _image_base64: &str,
        _screen_width: u32,
        _screen_height: u32,
        _tool_results: Option<Vec<ToolResult>>,
        _on_chunk: ChunkCallback,
    ) -> Result<LlmResponse, LlmError> {
        Err(LlmError::ApiError("Provider does not support native tool use".to_string()))
    }

    /// Check if this provider supports native tool use
    fn supports_tools(&self) -> bool {
        false
    }

    /// Check if the provider is reachable and operational.
    /// Default implementation returns NotConfigured error.
    async fn health_check(&self) -> Result<bool, LlmError> {
        Err(LlmError::NotConfigured)
    }

    /// List available models from the provider.
    /// Default implementation returns NotConfigured error.
    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        Err(LlmError::NotConfigured)
    }

    fn name(&self) -> &str;
}

/// Helper to convert conversation history to provider-specific message format.
/// Returns a Vec of tuples: (role, text_content, optional_image_base64)
/// The image_base64 is Arc-wrapped to avoid cloning large screenshot strings.
///
/// To reduce token usage, only the 2 most recent User messages retain their screenshots;
/// older screenshots are stripped (replaced with None).
pub fn history_to_messages(history: &ConversationHistory) -> Vec<(String, String, Option<Arc<String>>)> {
    // Count total User messages to determine which ones keep screenshots
    let user_msg_count = history.messages().filter(|m| matches!(m, Message::User { .. })).count();
    let keep_threshold = user_msg_count.saturating_sub(2);

    let mut user_index = 0usize;
    history
        .messages()
        .map(|msg| match msg {
            Message::User {
                instruction,
                screenshot_base64,
                ..
            } => {
                let keep_screenshot = user_index >= keep_threshold;
                user_index += 1;
                (
                    "user".to_string(),
                    instruction.clone(),
                    if keep_screenshot { screenshot_base64.clone() } else { None },
                )
            }
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
    build_system_prompt_for_tools_with_context(screen_width, screen_height, None, None, None)
}

/// Build system prompt for tool-based providers with optional task context and progress info
pub fn build_system_prompt_for_tools_with_context(
    screen_width: u32,
    screen_height: u32,
    instruction: Option<&str>,
    iteration: Option<u32>,
    max_iterations: Option<u32>,
) -> String {
    let mut prompt = format!(
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
    );

    if let Some(instr) = instruction {
        prompt.push_str(&format!("\n\n## Current Task\n{}", instr));
    }

    if let (Some(iter), Some(max)) = (iteration, max_iterations) {
        prompt.push_str(&format!("\n\n## Progress\nStep {} of {}.", iter, max));
        if max > 0 && iter > (max * 3) / 4 {
            prompt.push_str("\nYou are running low on steps. Focus on completing the task efficiently.");
        }
    }

    prompt
}

/// Build system prompt for JSON-based providers with optional task context and progress info
pub fn build_system_prompt_with_context(
    screen_width: u32,
    screen_height: u32,
    instruction: Option<&str>,
    iteration: Option<u32>,
    max_iterations: Option<u32>,
) -> String {
    let mut prompt = build_system_prompt(screen_width, screen_height);

    if let Some(instr) = instruction {
        prompt.push_str(&format!("\n\n## Current Task\n{}", instr));
    }

    if let (Some(iter), Some(max)) = (iteration, max_iterations) {
        prompt.push_str(&format!("\n\n## Progress\nStep {} of {}.", iter, max));
        if max > 0 && iter > (max * 3) / 4 {
            prompt.push_str("\nYou are running low on steps. Focus on completing the task efficiently.");
        }
    }

    prompt
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

11. Wait for element before proceeding:
    {{"action": "wait_for_element", "timeout_ms": 3000, "description": "page to load"}}
    Use when:
    - After clicking a button that triggers loading
    - After navigating to a new page
    - When an element might not be immediately visible
    Default timeout is 5000ms. Max is 10000ms.

12. Complete the task:
    {{"action": "complete", "message": "Task completed successfully"}}

13. Report an error or inability to proceed:
    {{"action": "error", "message": "Cannot find the required element"}}

14. Execute multiple actions in sequence (batch):
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

You may think briefly about what you see and what action to take, then respond with a JSON action.
Format: optional reasoning text, followed by the JSON object. Example:

I can see the search bar at the top of the page. I'll click on it to start typing.
{{"action": "click", "x": 540, "y": 35}}

If an action doesn't seem to work or the screen hasn't changed:
- Try slightly different coordinates (UI elements may have shifted)
- Try keyboard navigation instead of clicking a menu
- Scroll to find elements that may be off-screen
- Wait briefly for slow-loading UI: {{"action": "wait", "duration_ms": 2000}}
- If truly stuck after multiple attempts, report: {{"action": "error", "message": "description"}}

Your response must contain exactly one JSON action object."#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_token_metrics_tokens_per_second() {
        let metrics = TokenMetrics {
            input_tokens: 100,
            output_tokens: 50,
            total_duration: Duration::from_secs(2),
        };
        assert!((metrics.tokens_per_second() - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_token_metrics_zero_duration() {
        let metrics = TokenMetrics {
            input_tokens: 100,
            output_tokens: 50,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(metrics.tokens_per_second(), 0.0);
    }

    #[test]
    fn test_tool_use_serialization() {
        let tu = ToolUse {
            id: "tool_abc".to_string(),
            name: "click".to_string(),
            input: json!({"x": 100, "y": 200}),
        };
        let json_str = serde_json::to_string(&tu).unwrap();
        let parsed: ToolUse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.id, "tool_abc");
        assert_eq!(parsed.name, "click");
        assert_eq!(parsed.input["x"], 100);
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("id1".to_string(), "done".to_string());
        assert!(!result.is_error);
        assert_eq!(result.tool_use_id, "id1");
        assert_eq!(result.content, "done");
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("id2".to_string(), "failed".to_string());
        assert!(result.is_error);
        assert_eq!(result.content, "failed");
    }

    #[test]
    fn test_tool_result_to_json() {
        let result = ToolResult::success("id3".to_string(), "ok".to_string());
        let json = result.to_json();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["tool_use_id"], "id3");
        assert_eq!(json["is_error"], false);
        assert_eq!(json["content"], "ok");
    }

    #[test]
    fn test_build_tools_returns_expected_tools() {
        let tools = build_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"click"));
        assert!(names.contains(&"double_click"));
        assert!(names.contains(&"move"));
        assert!(names.contains(&"type"));
        assert!(names.contains(&"key"));
        assert!(names.contains(&"scroll"));
        assert!(names.contains(&"complete"));
        assert!(names.contains(&"error"));
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn test_build_system_prompt_contains_dimensions() {
        let prompt = build_system_prompt(1920, 1080);
        assert!(prompt.contains("1920x1080"));
    }

    #[test]
    fn test_build_system_prompt_for_tools_contains_dimensions() {
        let prompt = build_system_prompt_for_tools(2560, 1440);
        assert!(prompt.contains("2560x1440"));
    }

    #[test]
    fn test_llm_response_to_string_repr_text() {
        let resp = LlmResponse::Text("hello".to_string());
        assert_eq!(resp.to_string_repr(), "hello");
    }

    #[test]
    fn test_llm_response_to_string_repr_tool_use() {
        let resp = LlmResponse::ToolUse {
            tool_use: ToolUse {
                id: "id1".to_string(),
                name: "click".to_string(),
                input: json!({"x": 1}),
            },
            reasoning: None,
        };
        let repr = resp.to_string_repr();
        assert!(repr.contains("click"));
        assert!(repr.contains("id1"));
    }

    #[test]
    fn test_history_to_messages_user_message() {
        let mut history = ConversationHistory::new();
        history.add_user_message("Click the button", Some("img_data".to_string().into()), Some(1920), Some(1080));

        let messages = history_to_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "user");
        assert_eq!(messages[0].1, "Click the button");
        assert_eq!(messages[0].2.as_deref(), Some(&"img_data".to_string()));
    }

    #[test]
    fn test_history_to_messages_assistant_message() {
        let mut history = ConversationHistory::new();
        history.add_assistant_message(r#"{"action": "click", "x": 100, "y": 200}"#);

        let messages = history_to_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "assistant");
        assert!(messages[0].1.contains("click"));
        assert!(messages[0].2.is_none());
    }

    #[test]
    fn test_history_to_messages_tool_result_success() {
        let mut history = ConversationHistory::new();
        history.add_tool_result(true, Some("Clicked successfully".to_string()), None);

        let messages = history_to_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "user");
        assert!(messages[0].1.contains("successfully"));
        assert!(messages[0].1.contains("Clicked successfully"));
    }

    #[test]
    fn test_history_to_messages_tool_result_failure() {
        let mut history = ConversationHistory::new();
        history.add_tool_result(false, None, Some("Element not found".to_string()));

        let messages = history_to_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "user");
        assert!(messages[0].1.contains("failed"));
        assert!(messages[0].1.contains("Element not found"));
    }

    #[test]
    fn test_history_to_messages_mixed_conversation() {
        let mut history = ConversationHistory::new();
        history.add_user_message("Open browser", Some("screenshot1".to_string().into()), Some(1920), Some(1080));
        history.add_assistant_message(r#"{"action": "click", "x": 50, "y": 60}"#);
        history.add_tool_result(true, Some("Clicked".to_string()), None);
        history.add_user_message("Next step", Some("screenshot2".to_string().into()), Some(1920), Some(1080));

        let messages = history_to_messages(&history);
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].0, "user");
        assert_eq!(messages[1].0, "assistant");
        assert_eq!(messages[2].0, "user"); // tool result mapped to user role
        assert_eq!(messages[3].0, "user");
    }

    #[test]
    fn test_history_to_messages_empty() {
        let history = ConversationHistory::new();
        let messages = history_to_messages(&history);
        assert!(messages.is_empty());
    }

    #[test]
    fn test_build_system_prompt_contains_all_action_types() {
        let prompt = build_system_prompt(1920, 1080);
        assert!(prompt.contains("click"));
        assert!(prompt.contains("double_click"));
        assert!(prompt.contains("type"));
        assert!(prompt.contains("key"));
        assert!(prompt.contains("scroll"));
        assert!(prompt.contains("move"));
        assert!(prompt.contains("drag"));
        assert!(prompt.contains("triple_click"));
        assert!(prompt.contains("right_click"));
        assert!(prompt.contains("wait"));
        assert!(prompt.contains("wait_for_element"));
        assert!(prompt.contains("complete"));
        assert!(prompt.contains("error"));
        assert!(prompt.contains("batch"));
    }

    #[test]
    fn test_build_tools_schema_has_required_fields() {
        let tools = build_tools();
        let click_tool = tools.iter().find(|t| t.name == "click").unwrap();
        let required = click_tool.input_schema["required"].as_array().unwrap();
        let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(required_strs.contains(&"x"));
        assert!(required_strs.contains(&"y"));
    }
}
