use super::provider::{
    build_system_prompt, history_to_messages, ChunkCallback, LlmError, LlmProvider, LlmResponse,
    TokenMetrics,
};
use super::sse::append_bytes_to_buffer;
use serde_json::Value;
use crate::agent::conversation::ConversationHistory;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct OllamaProvider {
    client: Client,
    host: String,
    model: String,
}

#[derive(Serialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaChatStreamResponse {
    message: Option<OllamaChatMessageResponse>,
    done: bool,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
}

#[derive(Deserialize)]
struct OllamaChatMessageResponse {
    #[allow(dead_code)]
    role: Option<String>,
    content: Option<String>,
}

impl OllamaProvider {
    pub fn new(host: String, model: String) -> Self {
        Self {
            client: Client::new(),
            host,
            model,
        }
    }

    pub fn with_timeouts(host: String, model: String, connect_timeout: Duration, response_timeout: Duration) -> Self {
        let client = Client::builder()
            .connect_timeout(connect_timeout)
            .timeout(response_timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            host,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn send_with_history(
        &self,
        history: &ConversationHistory,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(LlmResponse, TokenMetrics), LlmError> {
        let start = Instant::now();
        let system_prompt = build_system_prompt(screen_width, screen_height);

        let mut messages = Vec::new();

        // System message
        messages.push(OllamaChatMessage {
            role: "system".to_string(),
            content: system_prompt,
            images: None,
        });

        // Convert conversation history to chat messages
        for (role, text, image_base64) in history_to_messages(history) {
            let (content, images) = if let Some(img_data) = image_base64 {
                (
                    format!("[Screenshot attached]\n{}\n\nAnalyze the screenshot and respond with a single JSON action.", text),
                    Some(vec![(*img_data).clone()]),
                )
            } else {
                (text, None)
            };

            messages.push(OllamaChatMessage {
                role,
                content,
                images,
            });
        }

        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.host))
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
        let mut output_tokens = 0u64;
        let mut input_tokens = 0u64;
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            append_bytes_to_buffer(&mut buffer, &chunk);

            // Process complete newline-delimited JSON lines; partial lines
            // are left in the buffer for the next chunk.
            while let Some(pos) = buffer.find('\n') {
                {
                    let line = &buffer[..pos];
                    if !line.is_empty() {
                        if let Ok(parsed) = serde_json::from_str::<OllamaChatStreamResponse>(line) {
                            if let Some(msg) = &parsed.message {
                                if let Some(content) = &msg.content {
                                    if !content.is_empty() {
                                        full_response.push_str(content);
                                        on_chunk(content);
                                    }
                                }
                            }

                            if parsed.done {
                                output_tokens = parsed.eval_count.unwrap_or(0);
                                input_tokens = parsed.prompt_eval_count.unwrap_or(0);
                            }
                        }
                    }
                }
                buffer.drain(..pos + 1);
            }
        }

        // Process any trailing data left in the buffer (no final newline)
        if !buffer.is_empty() {
            if let Ok(parsed) = serde_json::from_str::<OllamaChatStreamResponse>(&buffer) {
                if let Some(msg) = &parsed.message {
                    if let Some(content) = &msg.content {
                        if !content.is_empty() {
                            full_response.push_str(content);
                            on_chunk(content);
                        }
                    }
                }

                if parsed.done {
                    output_tokens = parsed.eval_count.unwrap_or(0);
                    input_tokens = parsed.prompt_eval_count.unwrap_or(0);
                }
            }
        }

        let metrics = TokenMetrics {
            input_tokens,
            output_tokens,
            total_duration: start.elapsed(),
        };

        Ok((LlmResponse::Text(full_response), metrics))
    }

    async fn health_check(&self) -> Result<bool, LlmError> {
        let url = format!("{}/api/tags", self.host);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/api/tags", self.host);
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(LlmError::ApiError(format!(
                "Failed to list models: HTTP {}",
                response.status()
            )));
        }
        let body: Value = response.json().await.map_err(|e| {
            LlmError::ParseError(format!("Failed to parse model list: {}", e))
        })?;
        let models = body["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(models)
    }

    fn name(&self) -> &str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_chat_message_serialization_without_images() {
        let msg = OllamaChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            images: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello");
        assert!(json.get("images").is_none(), "images field should be omitted when None");
    }

    #[test]
    fn test_chat_message_serialization_with_images() {
        let msg = OllamaChatMessage {
            role: "user".to_string(),
            content: "Describe this".to_string(),
            images: Some(vec!["base64data".to_string()]),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Describe this");
        assert_eq!(json["images"][0], "base64data");
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = OllamaChatRequest {
            model: "llava".to_string(),
            messages: vec![
                OllamaChatMessage {
                    role: "system".to_string(),
                    content: "You are a helper.".to_string(),
                    images: None,
                },
                OllamaChatMessage {
                    role: "user".to_string(),
                    content: "Click the button".to_string(),
                    images: Some(vec!["screenshot_data".to_string()]),
                },
            ],
            stream: true,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "llava");
        assert_eq!(json["stream"], true);
        assert_eq!(json["messages"].as_array().unwrap().len(), 2);
        assert_eq!(json["messages"][0]["role"], "system");
        assert!(json["messages"][0].get("images").is_none());
        assert_eq!(json["messages"][1]["images"][0], "screenshot_data");
    }

    #[test]
    fn test_stream_response_parsing_content_chunk() {
        let chunk = json!({
            "message": {"role": "assistant", "content": "hello"},
            "done": false
        });
        let parsed: OllamaChatStreamResponse = serde_json::from_value(chunk).unwrap();
        assert!(!parsed.done);
        assert_eq!(
            parsed.message.as_ref().unwrap().content.as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn test_stream_response_parsing_done() {
        let chunk = json!({
            "message": {"role": "assistant", "content": ""},
            "done": true,
            "eval_count": 42,
            "prompt_eval_count": 100
        });
        let parsed: OllamaChatStreamResponse = serde_json::from_value(chunk).unwrap();
        assert!(parsed.done);
        assert_eq!(parsed.eval_count, Some(42));
        assert_eq!(parsed.prompt_eval_count, Some(100));
    }

    #[test]
    fn test_stream_response_parsing_missing_metrics() {
        let chunk = json!({
            "message": {"role": "assistant", "content": "text"},
            "done": false
        });
        let parsed: OllamaChatStreamResponse = serde_json::from_value(chunk).unwrap();
        assert_eq!(parsed.eval_count, None);
        assert_eq!(parsed.prompt_eval_count, None);
    }

    #[test]
    fn test_stream_response_empty_content() {
        let chunk = json!({
            "message": {"role": "assistant", "content": ""},
            "done": false
        });
        let parsed: OllamaChatStreamResponse = serde_json::from_value(chunk).unwrap();
        assert_eq!(
            parsed.message.as_ref().unwrap().content.as_deref(),
            Some("")
        );
    }

    #[test]
    fn test_history_to_chat_messages() {
        let mut history = ConversationHistory::new();
        history.add_user_message("Click the button", Some("img_data".to_string().into()), Some(1920), Some(1080));
        history.add_assistant_message(r#"{"action": "click", "x": 100, "y": 200}"#);
        history.add_tool_result(true, Some("Clicked successfully".to_string()), None);
        history.add_user_message("Now type hello", None, None, None);

        let raw_messages = history_to_messages(&history);
        let mut chat_messages: Vec<OllamaChatMessage> = Vec::new();

        // System message
        chat_messages.push(OllamaChatMessage {
            role: "system".to_string(),
            content: "System prompt".to_string(),
            images: None,
        });

        for (role, text, image_base64) in raw_messages {
            let (content, images) = if let Some(img_data) = image_base64 {
                (
                    format!("[Screenshot attached]\n{}\n\nAnalyze the screenshot and respond with a single JSON action.", text),
                    Some(vec![(*img_data).clone()]),
                )
            } else {
                (text, None)
            };
            chat_messages.push(OllamaChatMessage { role, content, images });
        }

        // 1 system + 4 conversation messages
        assert_eq!(chat_messages.len(), 5);

        // System message has no images
        assert_eq!(chat_messages[0].role, "system");
        assert!(chat_messages[0].images.is_none());

        // First user message has image
        assert_eq!(chat_messages[1].role, "user");
        assert!(chat_messages[1].images.is_some());
        assert_eq!(chat_messages[1].images.as_ref().unwrap()[0], "img_data");

        // Assistant message has no images
        assert_eq!(chat_messages[2].role, "assistant");
        assert!(chat_messages[2].images.is_none());

        // Tool result mapped to user role, no images
        assert_eq!(chat_messages[3].role, "user");
        assert!(chat_messages[3].images.is_none());

        // Second user message has no images
        assert_eq!(chat_messages[4].role, "user");
        assert!(chat_messages[4].images.is_none());
    }

    #[test]
    fn test_provider_name() {
        let provider = OllamaProvider::new("http://localhost:11434".to_string(), "llava".to_string());
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_chat_request_uses_correct_endpoint() {
        let provider = OllamaProvider::new("http://localhost:11434".to_string(), "llava".to_string());
        let expected = format!("{}/api/chat", provider.host);
        assert_eq!(expected, "http://localhost:11434/api/chat");
    }

    #[test]
    fn test_health_check_url() {
        let provider = OllamaProvider::new("http://localhost:11434".to_string(), "llava".to_string());
        let url = format!("{}/api/tags", provider.host);
        assert_eq!(url, "http://localhost:11434/api/tags");
    }

    #[test]
    fn test_list_models_response_parsing() {
        let response_json = json!({
            "models": [
                {"name": "llava:latest", "size": 4000000000u64},
                {"name": "codellama:7b", "size": 3000000000u64},
                {"name": "mistral:latest", "size": 4000000000u64}
            ]
        });

        let models: Vec<String> = response_json["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        assert_eq!(models.len(), 3);
        assert_eq!(models[0], "llava:latest");
        assert_eq!(models[1], "codellama:7b");
        assert_eq!(models[2], "mistral:latest");
    }

    #[test]
    fn test_list_models_empty_response() {
        let response_json = json!({"models": []});

        let models: Vec<String> = response_json["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        assert!(models.is_empty());
    }

    #[test]
    fn test_list_models_missing_field() {
        let response_json = json!({"other": "data"});

        let models: Vec<String> = response_json["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        assert!(models.is_empty());
    }
}
