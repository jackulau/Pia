use super::provider::{
    build_system_prompt, build_system_prompt_for_tools, build_tools, ChunkCallback, LlmError,
    LlmProvider, LlmResponse, TokenMetrics, Tool, ToolUse, history_to_messages,
};
use super::sse::append_bytes_to_buffer;
use crate::agent::conversation::ConversationHistory;
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

        // Convert conversation history to Anthropic message format
        let messages: Vec<AnthropicMessage> = history_to_messages(history)
            .into_iter()
            .map(|(role, text, image_base64)| {
                let mut content = Vec::new();

                // Add image first if present (Anthropic prefers image before text)
                if let Some(img_data) = image_base64 {
                    content.push(AnthropicContent::Image {
                        source: ImageSource {
                            source_type: "base64".to_string(),
                            media_type: "image/png".to_string(),
                            data: img_data,
                        },
                    });
                }

                // Add text content
                let text_content = if role == "user" && content.iter().any(|c| matches!(c, AnthropicContent::Image { .. })) {
                    format!(
                        "User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.",
                        text
                    )
                } else {
                    text
                };
                content.push(AnthropicContent::Text { text: text_content });

                AnthropicMessage { role, content }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_anthropic_message_text_serializes_correctly() {
        let msg = AnthropicMessage {
            role: "user".to_string(),
            content: vec![AnthropicContent::Text {
                text: "Click the button".to_string(),
            }],
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        let content = json["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Click the button");
    }

    #[test]
    fn test_anthropic_message_image_serializes_correctly() {
        let msg = AnthropicMessage {
            role: "user".to_string(),
            content: vec![AnthropicContent::Image {
                source: ImageSource {
                    source_type: "base64".to_string(),
                    media_type: "image/png".to_string(),
                    data: Arc::new("abc123data".to_string()),
                },
            }],
        };
        let json = serde_json::to_value(&msg).unwrap();
        let content = json["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["type"], "base64");
        assert_eq!(content[0]["source"]["media_type"], "image/png");
        assert_eq!(content[0]["source"]["data"], "abc123data");
    }

    #[test]
    fn test_anthropic_message_mixed_image_and_text() {
        let msg = AnthropicMessage {
            role: "user".to_string(),
            content: vec![
                AnthropicContent::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: "image/png".to_string(),
                        data: Arc::new("screenshot_data".to_string()),
                    },
                },
                AnthropicContent::Text {
                    text: "User instruction: Click the button\n\nAnalyze the screenshot and respond with a single JSON action.".to_string(),
                },
            ],
        };
        let json = serde_json::to_value(&msg).unwrap();
        let content = json["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        // Image comes first (Anthropic prefers image before text)
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[1]["type"], "text");
        assert!(content[1]["text"].as_str().unwrap().contains("Click the button"));
    }

    #[test]
    fn test_anthropic_request_serializes_with_tools() {
        let tools = build_tools();
        let request = AnthropicRequest {
            model: "claude-sonnet-4-5-20250514".to_string(),
            max_tokens: 1024,
            system: "Test system prompt".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: vec![AnthropicContent::Text {
                    text: "test".to_string(),
                }],
            }],
            tools,
            stream: true,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-5-20250514");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["stream"], true);
        let tools = json["tools"].as_array().unwrap();
        assert!(!tools.is_empty());
        // Each tool has name, description, input_schema
        for tool in tools {
            assert!(tool["name"].is_string());
            assert!(tool["description"].is_string());
            assert!(tool["input_schema"].is_object());
        }
    }

    #[test]
    fn test_multi_turn_message_sequence_from_history() {
        // Create a multi-turn conversation history
        let mut history = ConversationHistory::new();
        history.add_user_message(
            "Click the button",
            Some(Arc::new("screenshot1".to_string())),
            Some(1920),
            Some(1080),
        );
        history.add_assistant_message(r#"{"action": "click", "x": 100, "y": 200}"#);
        history.add_tool_result(true, Some("Clicked at (100, 200)".to_string()), None);
        history.add_user_message(
            "Now type hello",
            Some(Arc::new("screenshot2".to_string())),
            Some(1920),
            Some(1080),
        );
        history.add_assistant_message(r#"{"action": "type", "text": "hello"}"#);
        history.add_tool_result(true, Some("Typed: hello".to_string()), None);

        // Convert to Anthropic message format
        let messages: Vec<AnthropicMessage> = history_to_messages(&history)
            .into_iter()
            .map(|(role, text, image_base64)| {
                let mut content = Vec::new();
                if let Some(img_data) = image_base64 {
                    content.push(AnthropicContent::Image {
                        source: ImageSource {
                            source_type: "base64".to_string(),
                            media_type: "image/png".to_string(),
                            data: img_data,
                        },
                    });
                }
                let text_content = if role == "user" && content.iter().any(|c| matches!(c, AnthropicContent::Image { .. })) {
                    format!("User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.", text)
                } else {
                    text
                };
                content.push(AnthropicContent::Text { text: text_content });
                AnthropicMessage { role, content }
            })
            .collect();

        // Verify the 6-message sequence
        assert_eq!(messages.len(), 6);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[2].role, "user"); // tool result
        assert_eq!(messages[3].role, "user");
        assert_eq!(messages[4].role, "assistant");
        assert_eq!(messages[5].role, "user"); // tool result

        // User messages with screenshots have 2 content blocks (image + text)
        assert_eq!(messages[0].content.len(), 2);
        assert_eq!(messages[3].content.len(), 2);

        // Assistant messages have 1 content block (text)
        assert_eq!(messages[1].content.len(), 1);

        // Tool result messages have 1 content block (text, no image)
        assert_eq!(messages[2].content.len(), 1);
    }

    #[test]
    fn test_tool_result_success_message_format() {
        let mut history = ConversationHistory::new();
        history.add_tool_result(true, Some("Clicked successfully".to_string()), None);

        let messages = history_to_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "user");
        assert!(messages[0].1.contains("successfully"));
        assert!(messages[0].1.contains("Clicked successfully"));
    }

    #[test]
    fn test_tool_result_error_message_format() {
        let mut history = ConversationHistory::new();
        history.add_tool_result(false, None, Some("Element not found".to_string()));

        let messages = history_to_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "user");
        assert!(messages[0].1.contains("failed"));
        assert!(messages[0].1.contains("Element not found"));
    }

    #[test]
    fn test_stream_event_content_block_tool_use_deserializes() {
        let json_str = r#"{"type": "content_block_start", "index": 0, "content_block": {"type": "tool_use", "id": "toolu_123", "name": "click"}}"#;
        let event: StreamEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.event_type, "content_block_start");
        assert_eq!(event.index, Some(0));
        match event.content_block {
            Some(ContentBlock::ToolUse { id, name }) => {
                assert_eq!(id, "toolu_123");
                assert_eq!(name, "click");
            }
            _ => panic!("Expected ToolUse content block"),
        }
    }

    #[test]
    fn test_stream_event_content_block_text_deserializes() {
        let json_str = r#"{"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": "I'll click the button"}}"#;
        let event: StreamEvent = serde_json::from_str(json_str).unwrap();
        match event.content_block {
            Some(ContentBlock::Text { text }) => {
                assert_eq!(text, "I'll click the button");
            }
            _ => panic!("Expected Text content block"),
        }
    }

    #[test]
    fn test_stream_event_content_delta_text_deserializes() {
        let json_str = r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "some text"}}"#;
        let event: StreamEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.event_type, "content_block_delta");
        let delta = event.delta.unwrap();
        assert_eq!(delta.text, Some("some text".to_string()));
    }

    #[test]
    fn test_stream_event_content_delta_partial_json_deserializes() {
        let json_str = r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "input_json_delta", "partial_json": "{\"x\": 100"}}"#;
        let event: StreamEvent = serde_json::from_str(json_str).unwrap();
        let delta = event.delta.unwrap();
        assert_eq!(delta.partial_json, Some("{\"x\": 100".to_string()));
    }

    #[test]
    fn test_stream_event_message_start_with_usage() {
        let json_str = r#"{"type": "message_start", "message": {"usage": {"input_tokens": 500, "output_tokens": 0}}}"#;
        let event: StreamEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.event_type, "message_start");
        let msg = event.message.unwrap();
        let usage = msg.usage.unwrap();
        assert_eq!(usage.input_tokens, Some(500));
        assert_eq!(usage.output_tokens, Some(0));
    }

    #[test]
    fn test_stream_event_message_delta_with_output_tokens() {
        let json_str = r#"{"type": "message_delta", "usage": {"output_tokens": 42}}"#;
        let event: StreamEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.event_type, "message_delta");
        let usage = event.usage.unwrap();
        assert_eq!(usage.output_tokens, Some(42));
    }

    #[test]
    fn test_anthropic_provider_new() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-sonnet-4-5-20250514".to_string());
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "claude-sonnet-4-5-20250514");
    }

    #[test]
    fn test_anthropic_provider_with_timeouts() {
        let provider = AnthropicProvider::with_timeouts(
            "test-key".to_string(),
            "claude-sonnet-4-5-20250514".to_string(),
            Duration::from_secs(5),
            Duration::from_secs(30),
        );
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.api_key, "test-key");
    }
}
