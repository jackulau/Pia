use super::provider::{
    build_system_prompt, ChunkCallback, LlmError, LlmProvider, LlmResponse, TokenMetrics,
    ToolResult, ToolUse,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

/// Tool definition for the Anthropic API
#[derive(Serialize)]
struct Tool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

/// Represents the computer use tool for controlling mouse and keyboard
fn get_computer_use_tools() -> Vec<Tool> {
    vec![Tool {
        name: "computer".to_string(),
        description: "Control the computer by performing mouse clicks, keyboard input, scrolling, and other actions. Analyze the screenshot to determine the correct coordinates and action to take.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["click", "double_click", "move", "type", "key", "scroll", "complete", "error"],
                    "description": "The type of action to perform"
                },
                "x": {
                    "type": "integer",
                    "description": "X coordinate for mouse actions (click, double_click, move, scroll)"
                },
                "y": {
                    "type": "integer",
                    "description": "Y coordinate for mouse actions (click, double_click, move, scroll)"
                },
                "button": {
                    "type": "string",
                    "enum": ["left", "right", "middle"],
                    "description": "Mouse button for click action (default: left)"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type for 'type' action"
                },
                "key": {
                    "type": "string",
                    "description": "Key to press for 'key' action (e.g., 'enter', 'tab', 'escape', 'a', 'b')"
                },
                "modifiers": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["ctrl", "alt", "shift", "meta"] },
                    "description": "Modifier keys to hold during 'key' action"
                },
                "direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"],
                    "description": "Scroll direction for 'scroll' action"
                },
                "amount": {
                    "type": "integer",
                    "description": "Scroll amount for 'scroll' action (default: 3)"
                },
                "message": {
                    "type": "string",
                    "description": "Message for 'complete' or 'error' action"
                }
            },
            "required": ["action"]
        }),
    }]
}

#[derive(Serialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        content: String,
    },
}

#[derive(Serialize, Clone)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Deserialize, Debug)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    delta: Option<ContentDelta>,
    #[serde(default)]
    content_block: Option<ContentBlock>,
    #[serde(default)]
    message: Option<MessageInfo>,
    #[serde(default)]
    usage: Option<UsageInfo>,
}

#[derive(Deserialize, Debug)]
struct ContentDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
    partial_json: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
struct MessageInfo {
    usage: Option<UsageInfo>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct UsageInfo {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

/// Tracks the current state of a tool_use block being streamed
#[derive(Debug, Default)]
struct ToolUseBuilder {
    id: Option<String>,
    name: Option<String>,
    input_json: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn send_with_image(
        &self,
        instruction: &str,
        image_base64: &str,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(String, TokenMetrics), LlmError> {
        let start = Instant::now();
        let system_prompt = build_system_prompt(screen_width, screen_height);

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            system: system_prompt,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: vec![
                    AnthropicContent::Image {
                        source: ImageSource {
                            source_type: "base64".to_string(),
                            media_type: "image/png".to_string(),
                            data: image_base64.to_string(),
                        },
                    },
                    AnthropicContent::Text {
                        text: format!(
                            "User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.",
                            instruction
                        ),
                    },
                ],
            }],
            stream: true,
            tools: None,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(error_text));
        }

        let mut stream = response.bytes_stream();
        let mut full_response = String::new();
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE events
            while let Some(pos) = buffer.find("\n\n") {
                let event_str = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_str.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                            match event.event_type.as_str() {
                                "message_start" => {
                                    if let Some(msg) = event.message {
                                        if let Some(usage) = msg.usage {
                                            input_tokens = usage.input_tokens.unwrap_or(0);
                                        }
                                    }
                                }
                                "content_block_delta" => {
                                    if let Some(delta) = event.delta {
                                        if let Some(text) = delta.text {
                                            full_response.push_str(&text);
                                            on_chunk(&text);
                                        }
                                    }
                                }
                                "message_delta" => {
                                    if let Some(usage) = event.usage {
                                        output_tokens = usage.output_tokens.unwrap_or(0);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        let metrics = TokenMetrics {
            input_tokens,
            output_tokens,
            total_duration: start.elapsed(),
        };

        Ok((full_response, metrics))
    }

    async fn send_with_tools(
        &self,
        instruction: &str,
        image_base64: &str,
        screen_width: u32,
        screen_height: u32,
        tool_results: Option<Vec<ToolResult>>,
        on_chunk: ChunkCallback,
    ) -> Result<LlmResponse, LlmError> {
        let start = Instant::now();
        let system_prompt = build_tool_system_prompt(screen_width, screen_height);

        // Build messages based on whether we have tool results
        let messages = if let Some(results) = tool_results {
            // Continue conversation with tool results
            // Note: In a full implementation, we'd maintain conversation history
            // For now, we send the tool results as a user message
            vec![
                AnthropicMessage {
                    role: "user".to_string(),
                    content: vec![
                        AnthropicContent::Image {
                            source: ImageSource {
                                source_type: "base64".to_string(),
                                media_type: "image/png".to_string(),
                                data: image_base64.to_string(),
                            },
                        },
                        AnthropicContent::Text {
                            text: format!("User instruction: {}", instruction),
                        },
                    ],
                },
                AnthropicMessage {
                    role: "user".to_string(),
                    content: results
                        .into_iter()
                        .map(|r| AnthropicContent::ToolResult {
                            tool_use_id: r.tool_use_id,
                            is_error: if r.is_error { Some(true) } else { None },
                            content: r.content,
                        })
                        .collect(),
                },
            ]
        } else {
            // Initial request
            vec![AnthropicMessage {
                role: "user".to_string(),
                content: vec![
                    AnthropicContent::Image {
                        source: ImageSource {
                            source_type: "base64".to_string(),
                            media_type: "image/png".to_string(),
                            data: image_base64.to_string(),
                        },
                    },
                    AnthropicContent::Text {
                        text: format!(
                            "User instruction: {}\n\nAnalyze the screenshot and use the computer tool to perform the next action.",
                            instruction
                        ),
                    },
                ],
            }]
        };

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            system: system_prompt,
            messages,
            stream: true,
            tools: Some(get_computer_use_tools()),
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "computer-use-2024-10-22")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(error_text));
        }

        let mut stream = response.bytes_stream();
        let mut text_content = String::new();
        let mut tool_uses: Vec<ToolUse> = Vec::new();
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut stop_reason: Option<String> = None;
        let mut buffer = String::new();

        // Track current tool_use block being built
        let mut current_tool_builder: Option<ToolUseBuilder> = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE events
            while let Some(pos) = buffer.find("\n\n") {
                let event_str = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_str.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                            match event.event_type.as_str() {
                                "message_start" => {
                                    if let Some(msg) = event.message {
                                        if let Some(usage) = msg.usage {
                                            input_tokens = usage.input_tokens.unwrap_or(0);
                                        }
                                    }
                                }
                                "content_block_start" => {
                                    if let Some(block) = event.content_block {
                                        match block.block_type.as_str() {
                                            "text" => {
                                                // Text block starting
                                                if let Some(text) = block.text {
                                                    text_content.push_str(&text);
                                                    on_chunk(&text);
                                                }
                                            }
                                            "tool_use" => {
                                                // Tool use block starting
                                                current_tool_builder = Some(ToolUseBuilder {
                                                    id: block.id,
                                                    name: block.name,
                                                    input_json: String::new(),
                                                });
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                "content_block_delta" => {
                                    if let Some(delta) = event.delta {
                                        // Check delta type
                                        match delta.delta_type.as_deref() {
                                            Some("text_delta") => {
                                                if let Some(text) = delta.text {
                                                    text_content.push_str(&text);
                                                    on_chunk(&text);
                                                }
                                            }
                                            Some("input_json_delta") => {
                                                if let Some(json_part) = delta.partial_json {
                                                    if let Some(ref mut builder) =
                                                        current_tool_builder
                                                    {
                                                        builder.input_json.push_str(&json_part);
                                                    }
                                                }
                                            }
                                            _ => {
                                                // Fallback for text
                                                if let Some(text) = delta.text {
                                                    text_content.push_str(&text);
                                                    on_chunk(&text);
                                                }
                                            }
                                        }
                                    }
                                }
                                "content_block_stop" => {
                                    // Finalize current tool_use if any
                                    if let Some(builder) = current_tool_builder.take() {
                                        if let (Some(id), Some(name)) = (builder.id, builder.name) {
                                            let input: serde_json::Value =
                                                serde_json::from_str(&builder.input_json)
                                                    .unwrap_or(json!({}));
                                            tool_uses.push(ToolUse { id, name, input });
                                        }
                                    }
                                }
                                "message_delta" => {
                                    if let Some(usage) = event.usage {
                                        output_tokens = usage.output_tokens.unwrap_or(0);
                                    }
                                    if let Some(msg) = event.message {
                                        stop_reason = msg.stop_reason;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        let metrics = TokenMetrics {
            input_tokens,
            output_tokens,
            total_duration: start.elapsed(),
        };

        Ok(LlmResponse {
            text: if text_content.is_empty() {
                None
            } else {
                Some(text_content)
            },
            tool_uses,
            metrics,
            stop_reason,
        })
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "anthropic"
    }
}

/// Build system prompt for tool-based interactions
fn build_tool_system_prompt(screen_width: u32, screen_height: u32) -> String {
    format!(
        r#"You are a computer use agent that controls a computer to complete user tasks.

Screen dimensions: {screen_width}x{screen_height} pixels

You have access to the 'computer' tool which allows you to:
- Click at specific coordinates (action: "click")
- Double-click (action: "double_click")
- Move the mouse (action: "move")
- Type text (action: "type")
- Press keys with optional modifiers (action: "key")
- Scroll in any direction (action: "scroll")
- Report task completion (action: "complete")
- Report errors (action: "error")

Guidelines:
1. Analyze the screenshot carefully to identify UI elements
2. Use precise coordinates that match visible elements
3. Wait for UI feedback between actions (the system handles timing)
4. Use "complete" with a message when the task is done
5. Use "error" if you cannot proceed or need clarification

Always use the computer tool to perform actions - do not output raw JSON."#
    )
}
