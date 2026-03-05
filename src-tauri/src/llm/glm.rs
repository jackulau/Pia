use super::provider::{
    build_system_prompt, history_to_messages, ChunkCallback, LlmError, LlmProvider, LlmResponse, TokenMetrics,
};
use crate::agent::conversation::ConversationHistory;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

pub struct GlmProvider {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct GlmRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<GlmMessage>,
    stream: bool,
    stream_options: StreamOptions,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct GlmMessage {
    role: String,
    content: GlmContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GlmContent {
    Text(String),
    Parts(Vec<GlmPart>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum GlmPart {
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

impl GlmProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for GlmProvider {
    async fn send_with_history(
        &self,
        history: &ConversationHistory,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(LlmResponse, TokenMetrics), LlmError> {
        let start = Instant::now();
        let system_prompt = build_system_prompt(screen_width, screen_height);

        // Build messages from conversation history
        let mut messages = vec![GlmMessage {
            role: "system".to_string(),
            content: GlmContent::Text(system_prompt),
        }];

        for (role, text, image_base64) in history_to_messages(history) {
            let content = if let Some(img_data) = image_base64 {
                GlmContent::Parts(vec![
                    GlmPart::ImageUrl {
                        image_url: ImageUrl {
                            url: format!("data:image/png;base64,{}", img_data),
                        },
                    },
                    GlmPart::Text {
                        text: format!(
                            "User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.",
                            text
                        ),
                    },
                ])
            } else {
                GlmContent::Text(text)
            };

            messages.push(GlmMessage { role, content });
        }

        let request = GlmRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages,
            stream: true,
            stream_options: StreamOptions {
                include_usage: true,
            },
        };

        let response = self
            .client
            .post("https://open.bigmodel.cn/api/paas/v4/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

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
        let response = self
            .client
            .get("https://open.bigmodel.cn/api/paas/v4/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let response = self
            .client
            .get("https://open.bigmodel.cn/api/paas/v4/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
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
        "glm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_provider_name() {
        let provider = GlmProvider::new("test-key".to_string(), "glm-4v".to_string());
        assert_eq!(provider.name(), "glm");
    }

    #[test]
    fn test_glm_request_serialization() {
        let request = GlmRequest {
            model: "glm-4v".to_string(),
            max_tokens: 1024,
            messages: vec![GlmMessage {
                role: "system".to_string(),
                content: GlmContent::Text("You are an assistant.".to_string()),
            }],
            stream: true,
            stream_options: StreamOptions {
                include_usage: true,
            },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "glm-4v");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["stream"], true);
        assert_eq!(json["stream_options"]["include_usage"], true);
        assert_eq!(json["messages"].as_array().unwrap().len(), 1);
        assert_eq!(json["messages"][0]["role"], "system");
    }

    #[test]
    fn test_glm_message_text_content() {
        let msg = GlmMessage {
            role: "user".to_string(),
            content: GlmContent::Text("Hello".to_string()),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello");
    }

    #[test]
    fn test_glm_message_parts_content() {
        let msg = GlmMessage {
            role: "user".to_string(),
            content: GlmContent::Parts(vec![
                GlmPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "data:image/png;base64,abc123".to_string(),
                    },
                },
                GlmPart::Text {
                    text: "Describe this image".to_string(),
                },
            ]),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        let parts = json["content"].as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["type"], "image_url");
        assert_eq!(parts[0]["image_url"]["url"], "data:image/png;base64,abc123");
        assert_eq!(parts[1]["type"], "text");
        assert_eq!(parts[1]["text"], "Describe this image");
    }

    #[test]
    fn test_stream_chunk_parsing() {
        let chunk_json = json!({
            "choices": [
                {"delta": {"content": "hello"}}
            ]
        });
        let parsed: StreamChunk = serde_json::from_value(chunk_json).unwrap();
        assert_eq!(parsed.choices.len(), 1);
        assert_eq!(
            parsed.choices[0].delta.as_ref().unwrap().content.as_deref(),
            Some("hello")
        );
        assert!(parsed.usage.is_none());
    }

    #[test]
    fn test_stream_chunk_with_usage() {
        let chunk_json = json!({
            "choices": [
                {"delta": {"content": ""}}
            ],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50
            }
        });
        let parsed: StreamChunk = serde_json::from_value(chunk_json).unwrap();
        let usage = parsed.usage.unwrap();
        assert_eq!(usage.prompt_tokens, Some(100));
        assert_eq!(usage.completion_tokens, Some(50));
    }

    #[test]
    fn test_stream_chunk_empty_delta() {
        let chunk_json = json!({
            "choices": [
                {"delta": {}}
            ]
        });
        let parsed: StreamChunk = serde_json::from_value(chunk_json).unwrap();
        assert!(parsed.choices[0].delta.as_ref().unwrap().content.is_none());
    }

    #[test]
    fn test_stream_chunk_null_delta() {
        let chunk_json = json!({
            "choices": [
                {"delta": null}
            ]
        });
        let parsed: StreamChunk = serde_json::from_value(chunk_json).unwrap();
        assert!(parsed.choices[0].delta.is_none());
    }

    #[test]
    fn test_list_models_response_parsing() {
        let response_json = json!({
            "data": [
                {"id": "glm-4v", "object": "model"},
                {"id": "glm-4", "object": "model"},
                {"id": "glm-3-turbo", "object": "model"}
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
        assert_eq!(models[0], "glm-4v");
        assert_eq!(models[1], "glm-4");
        assert_eq!(models[2], "glm-3-turbo");
    }

    #[test]
    fn test_list_models_empty_response() {
        let response_json = json!({"data": []});

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
        let response_json = json!({"other": "data"});

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
    fn test_list_models_missing_id_field() {
        let response_json = json!({
            "data": [
                {"id": "glm-4v"},
                {"name": "no-id-model"},
                {"id": "glm-4"}
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

        // The model without an "id" field should be filtered out
        assert_eq!(models.len(), 2);
        assert_eq!(models[0], "glm-4v");
        assert_eq!(models[1], "glm-4");
    }

    #[test]
    fn test_history_to_glm_messages() {
        let mut history = ConversationHistory::new();
        history.add_user_message(
            "Click the button",
            Some("img_data".to_string().into()),
            Some(1920),
            Some(1080),
        );
        history.add_assistant_message(r#"{"action": "click", "x": 100, "y": 200}"#);
        history.add_tool_result(true, Some("Clicked successfully".to_string()), None);
        history.add_user_message("Now type hello", None, None, None);

        let raw_messages = history_to_messages(&history);
        let mut messages: Vec<GlmMessage> = vec![GlmMessage {
            role: "system".to_string(),
            content: GlmContent::Text("System prompt".to_string()),
        }];

        for (role, text, image_base64) in raw_messages {
            let content = if let Some(img_data) = image_base64 {
                GlmContent::Parts(vec![
                    GlmPart::ImageUrl {
                        image_url: ImageUrl {
                            url: format!("data:image/png;base64,{}", img_data),
                        },
                    },
                    GlmPart::Text {
                        text: format!(
                            "User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.",
                            text
                        ),
                    },
                ])
            } else {
                GlmContent::Text(text)
            };

            messages.push(GlmMessage { role, content });
        }

        // 1 system + 4 conversation messages
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(messages[3].role, "user"); // tool result mapped to user role
        assert_eq!(messages[4].role, "user");

        // Verify first user message has image parts
        let first_user_json = serde_json::to_value(&messages[1]).unwrap();
        let parts = first_user_json["content"].as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["type"], "image_url");

        // Verify assistant message is plain text
        let assistant_json = serde_json::to_value(&messages[2]).unwrap();
        assert!(assistant_json["content"].is_string());

        // Verify last user message without image is plain text
        let last_user_json = serde_json::to_value(&messages[4]).unwrap();
        assert!(last_user_json["content"].is_string());
    }

    #[test]
    fn test_glm_api_base_url() {
        // Verify the GLM API base URL and endpoints are correct
        let base_url = "https://open.bigmodel.cn/api/paas/v4";
        let chat_url = format!("{}/chat/completions", base_url);
        let models_url = format!("{}/models", base_url);
        assert_eq!(chat_url, "https://open.bigmodel.cn/api/paas/v4/chat/completions");
        assert_eq!(models_url, "https://open.bigmodel.cn/api/paas/v4/models");
    }

    #[test]
    fn test_authorization_header_format() {
        let api_key = "test-api-key-12345";
        let header = format!("Bearer {}", api_key);
        assert_eq!(header, "Bearer test-api-key-12345");
    }
}
