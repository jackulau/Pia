use super::provider::{
    build_system_prompt, history_to_messages, ChunkCallback, LlmError, LlmProvider, LlmResponse,
    TokenMetrics,
};
use serde_json::Value;
use crate::agent::conversation::ConversationHistory;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct OpenAICompatibleProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: ChatContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ChatContent {
    Text(String),
    Parts(Vec<ChatPart>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ChatPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<UsageInfo>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Option<DeltaContent>,
}

#[derive(Deserialize)]
struct DeltaContent {
    content: Option<String>,
}

#[derive(Deserialize)]
struct UsageInfo {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
}

impl OpenAICompatibleProvider {
    pub fn new(base_url: String, api_key: Option<String>, model: String, temperature: Option<f32>) -> Self {
        // Strip trailing slash for consistent URL building
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            client: Client::new(),
            base_url,
            api_key,
            model,
            temperature,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAICompatibleProvider {
    async fn send_with_history(
        &self,
        history: &ConversationHistory,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(LlmResponse, TokenMetrics), LlmError> {
        let start = Instant::now();
        let system_prompt = build_system_prompt(screen_width, screen_height);

        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(system_prompt),
        }];

        for (role, text, image_base64) in history_to_messages(history) {
            let content = if let Some(img_data) = image_base64 {
                ChatContent::Parts(vec![
                    ChatPart::ImageUrl {
                        image_url: ImageUrl {
                            url: format!("data:image/png;base64,{}", img_data),
                        },
                    },
                    ChatPart::Text {
                        text: format!(
                            "User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.",
                            text
                        ),
                    },
                ])
            } else {
                ChatContent::Text(text)
            };

            messages.push(ChatMessage { role, content });
        }

        let request = ChatRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages,
            stream: true,
            temperature: self.temperature,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(ref api_key) = self.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req_builder.json(&request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(error_text));
        }

        let mut stream = response.bytes_stream();
        let mut full_response = String::with_capacity(4096);
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                if let Some(data) = buffer[..pos].strip_prefix("data: ") {
                    if data != "[DONE]" {
                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                            for choice in chunk.choices {
                                if let Some(delta) = choice.delta {
                                    if let Some(content) = delta.content {
                                        full_response.push_str(&content);
                                        on_chunk(&content);
                                    }
                                }
                            }

                            if let Some(usage) = chunk.usage {
                                input_tokens = usage.prompt_tokens.unwrap_or(0);
                                output_tokens = usage.completion_tokens.unwrap_or(0);
                            }
                        }
                    }
                }
                buffer.drain(..pos + 1);
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
        let url = format!("{}/v1/models", self.base_url);
        let mut req = self.client.get(&url);
        if let Some(ref api_key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }
        let response = req.send().await?;
        Ok(response.status().is_success())
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/v1/models", self.base_url);
        let mut req = self.client.get(&url);
        if let Some(ref api_key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }
        let response = req.send().await?;
        if !response.status().is_success() {
            return Err(LlmError::ApiError(format!(
                "Failed to list models: HTTP {}",
                response.status()
            )));
        }
        let body: Value = response.json().await.map_err(|e| {
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
        "openai-compatible"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_strips_trailing_slash() {
        let provider =
            OpenAICompatibleProvider::new("http://localhost:1234/".to_string(), None, "model".to_string(), None);
        assert_eq!(provider.base_url, "http://localhost:1234");
    }

    #[test]
    fn test_new_no_trailing_slash() {
        let provider =
            OpenAICompatibleProvider::new("http://localhost:1234".to_string(), None, "model".to_string(), None);
        assert_eq!(provider.base_url, "http://localhost:1234");
    }

    #[test]
    fn test_new_with_api_key() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:1234".to_string(),
            Some("sk-test-key".to_string()),
            "gpt-4".to_string(),
            None,
        );
        assert_eq!(provider.api_key, Some("sk-test-key".to_string()));
        assert_eq!(provider.model, "gpt-4");
    }

    #[test]
    fn test_new_without_api_key() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:11434".to_string(),
            None,
            "llama3".to_string(),
            None,
        );
        assert_eq!(provider.api_key, None);
        assert_eq!(provider.model, "llama3");
    }

    #[test]
    fn test_name() {
        let provider =
            OpenAICompatibleProvider::new("http://localhost:1234".to_string(), None, "model".to_string(), None);
        assert_eq!(provider.name(), "openai-compatible");
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "test-model".to_string(),
            max_tokens: 1024,
            messages: vec![ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text("Hello".to_string()),
            }],
            stream: true,
            temperature: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "test-model");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["stream"], true);
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(json["messages"][0]["content"], "Hello");
    }

    #[test]
    fn test_multipart_content_serialization() {
        let content = ChatContent::Parts(vec![
            ChatPart::ImageUrl {
                image_url: ImageUrl {
                    url: "data:image/png;base64,abc123".to_string(),
                },
            },
            ChatPart::Text {
                text: "Describe this image".to_string(),
            },
        ]);

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json[0]["type"], "image_url");
        assert_eq!(json[0]["image_url"]["url"], "data:image/png;base64,abc123");
        assert_eq!(json[1]["type"], "text");
        assert_eq!(json[1]["text"], "Describe this image");
    }

    #[test]
    fn test_stream_chunk_deserialization() {
        let json = r#"{"choices":[{"delta":{"content":"Hello"}}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(
            chunk.choices[0].delta.as_ref().unwrap().content.as_ref().unwrap(),
            "Hello"
        );
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn test_stream_chunk_with_usage() {
        let json = r#"{"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":20}}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        let usage = chunk.usage.unwrap();
        assert_eq!(usage.prompt_tokens, Some(10));
        assert_eq!(usage.completion_tokens, Some(20));
    }

    #[test]
    fn test_health_check_url() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:1234".to_string(),
            None,
            "model".to_string(),
            None,
        );
        let url = format!("{}/v1/models", provider.base_url);
        assert_eq!(url, "http://localhost:1234/v1/models");
    }

    #[test]
    fn test_health_check_url_trailing_slash_stripped() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:1234/".to_string(),
            None,
            "model".to_string(),
            None,
        );
        let url = format!("{}/v1/models", provider.base_url);
        assert_eq!(url, "http://localhost:1234/v1/models");
    }

    #[test]
    fn test_list_models_response_parsing() {
        let response_json = serde_json::json!({
            "data": [
                {"id": "gpt-4", "object": "model"},
                {"id": "gpt-3.5-turbo", "object": "model"},
                {"id": "llama3", "object": "model"}
            ]
        });

        let models: Vec<String> = response_json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        assert_eq!(models.len(), 3);
        assert_eq!(models[0], "gpt-4");
        assert_eq!(models[1], "gpt-3.5-turbo");
        assert_eq!(models[2], "llama3");
    }

    #[test]
    fn test_list_models_empty_data() {
        let response_json = serde_json::json!({"data": []});

        let models: Vec<String> = response_json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        assert!(models.is_empty());
    }

    #[test]
    fn test_list_models_missing_data_field() {
        let response_json = serde_json::json!({"error": "unauthorized"});

        let models: Vec<String> = response_json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        assert!(models.is_empty());
    }
}
