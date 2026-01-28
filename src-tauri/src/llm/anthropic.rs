use super::provider::{
    build_system_prompt_for_tools, build_tools, ChunkCallback, LlmError, LlmProvider, LlmResponse,
    Tool, TokenMetrics, ToolUse,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
}

#[derive(Serialize)]
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
    ) -> Result<(LlmResponse, TokenMetrics), LlmError> {
        let start = Instant::now();
        let system_prompt = build_system_prompt_for_tools(screen_width, screen_height);
        let tools = build_tools();

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
                            "User instruction: {}\n\nAnalyze the screenshot and use the appropriate tool.",
                            instruction
                        ),
                    },
                ],
            }],
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

    fn name(&self) -> &str {
        "anthropic"
    }
}
