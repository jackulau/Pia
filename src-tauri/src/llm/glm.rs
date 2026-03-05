use super::provider::{
    build_system_prompt, history_to_messages, ChunkCallback, LlmError, LlmProvider, LlmResponse, TokenMetrics,
};
use crate::agent::conversation::ConversationHistory;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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

    fn name(&self) -> &str {
        "glm"
    }
}
