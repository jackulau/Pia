use super::provider::{
    build_system_prompt_for_tools, build_tools, ChunkCallback, LlmError,
    LlmProvider, LlmResponse, TokenMetrics, Tool, ToolUse,
};
use super::sse::append_bytes_to_buffer;
use crate::agent::conversation::{ConversationHistory, Message};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    tools: Vec<Tool>,
    stream: bool,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUseBlock {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResultBlock {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        content: String,
    },
}

#[derive(Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: Arc<String>,
}

#[derive(Deserialize, Debug)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    content_block: Option<ContentBlock>,
    #[serde(default)]
    delta: Option<ContentDelta>,
    #[serde(default)]
    message: Option<MessageInfo>,
    #[serde(default)]
    usage: Option<UsageInfo>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String },
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Deserialize, Debug)]
struct ContentDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    partial_json: Option<String>,
}

#[derive(Deserialize, Debug)]
struct MessageInfo {
    usage: Option<UsageInfo>,
}

#[derive(Deserialize, Debug)]
struct UsageInfo {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub fn with_timeouts(api_key: String, model: String, connect_timeout: Duration, response_timeout: Duration) -> Self {
        let client = Client::builder()
            .connect_timeout(connect_timeout)
            .timeout(response_timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            api_key,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn send_with_history(
        &self,
        history: &ConversationHistory,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(LlmResponse, TokenMetrics), LlmError> {
        let start = Instant::now();
        let system_prompt = build_system_prompt_for_tools(screen_width, screen_height);
        let tools = build_tools();

        // Convert conversation history to Anthropic message format with native content blocks
        let messages: Vec<AnthropicMessage> = history
            .messages()
            .map(|msg| match msg {
                Message::User {
                    instruction,
                    screenshot_base64,
                    ..
                } => {
                    let mut content = Vec::new();
                    if let Some(img_data) = screenshot_base64 {
                        content.push(AnthropicContent::Image {
                            source: ImageSource {
                                source_type: "base64".to_string(),
                                media_type: "image/png".to_string(),
                                data: img_data.clone(),
                            },
                        });
                    }
                    let text_content = if !content.is_empty() {
                        format!(
                            "User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.",
                            instruction
                        )
                    } else {
                        instruction.clone()
                    };
                    content.push(AnthropicContent::Text { text: text_content });
                    AnthropicMessage {
                        role: "user".to_string(),
                        content,
                    }
                }
                Message::Assistant { content: text } => AnthropicMessage {
                    role: "assistant".to_string(),
                    content: vec![AnthropicContent::Text { text: text.clone() }],
                },
                Message::AssistantToolUse {
                    tool_use_id,
                    tool_name,
                    tool_input,
                    text,
                } => {
                    let mut content = Vec::new();
                    if let Some(t) = text {
                        content.push(AnthropicContent::Text { text: t.clone() });
                    }
                    content.push(AnthropicContent::ToolUseBlock {
                        id: tool_use_id.clone(),
                        name: tool_name.clone(),
                        input: tool_input.clone(),
                    });
                    AnthropicMessage {
                        role: "assistant".to_string(),
                        content,
                    }
                }
                Message::ToolResult {
                    success,
                    tool_use_id,
                    message,
                    error,
                } => {
                    if let Some(id) = tool_use_id {
                        let result_text = if *success {
                            message
                                .as_deref()
                                .unwrap_or("Action executed successfully")
                                .to_string()
                        } else {
                            error.as_deref().unwrap_or("Action failed").to_string()
                        };
                        AnthropicMessage {
                            role: "user".to_string(),
                            content: vec![AnthropicContent::ToolResultBlock {
                                tool_use_id: id.clone(),
                                is_error: if *success { None } else { Some(true) },
                                content: result_text,
                            }],
                        }
                    } else {
                        // Fallback for tool results without ID (legacy)
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
                        AnthropicMessage {
                            role: "user".to_string(),
                            content: vec![AnthropicContent::Text { text }],
                        }
                    }
                }
            })
            .collect();

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            system: system_prompt,
            messages,
            tools,
            stream: true,
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
        // Pre-allocate response buffer with typical response size (~4KB)
        let mut full_response = String::with_capacity(4096);
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut buffer = String::new();

        // Track tool_use blocks as they stream
        let mut current_tool_id: Option<String> = None;
        let mut current_tool_name: Option<String> = None;
        let mut current_tool_input = String::new();
        let mut text_response = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            append_bytes_to_buffer(&mut buffer, &chunk);

            // Process complete SSE events using zero-allocation slicing
            while let Some(pos) = buffer.find("\n\n") {
                // Process event in-place before draining
                for line in buffer[..pos].lines() {
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
                                    if let Some(content_block) = event.content_block {
                                        match content_block {
                                            ContentBlock::ToolUse { id, name } => {
                                                current_tool_id = Some(id);
                                                current_tool_name = Some(name.clone());
                                                current_tool_input.clear();
                                                on_chunk(&format!("[Using tool: {}]", name));
                                            }
                                            ContentBlock::Text { text } => {
                                                text_response.push_str(&text);
                                                on_chunk(&text);
                                            }
                                        }
                                    }
                                }
                                "content_block_delta" => {
                                    if let Some(delta) = event.delta {
                                        // Handle text delta
                                        if let Some(text) = delta.text {
                                            text_response.push_str(&text);
                                            on_chunk(&text);
                                        }
                                        // Handle tool input JSON delta
                                        if let Some(partial_json) = delta.partial_json {
                                            current_tool_input.push_str(&partial_json);
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
                // Drain processed event from buffer (zero-allocation)
                buffer.drain(..pos + 2);
            }
        }

        let metrics = TokenMetrics {
            input_tokens,
            output_tokens,
            total_duration: start.elapsed(),
        };

        // Return tool_use if we received one, otherwise return text
        if let (Some(id), Some(name)) = (current_tool_id, current_tool_name) {
            let input: serde_json::Value = serde_json::from_str(&current_tool_input)
                .unwrap_or_else(|_| serde_json::json!({}));

            Ok((
                LlmResponse::ToolUse(ToolUse { id, name, input }),
                metrics,
            ))
        } else {
            Ok((LlmResponse::Text(text_response), metrics))
        }
    }

    async fn health_check(&self) -> Result<bool, LlmError> {
        let response = self
            .client
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let response = self
            .client
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(LlmError::ApiError(format!(
                "Failed to list models: HTTP {}",
                response.status()
            )));
        }
        let body: serde_json::Value = response.json().await.map_err(|e| {
            LlmError::ParseError(format!("Failed to parse model list: {}", e))
        })?;
        let models = body["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(models)
    }

    fn name(&self) -> &str {
        "anthropic"
    }
}
