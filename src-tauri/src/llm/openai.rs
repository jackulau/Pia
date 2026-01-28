use super::provider::{
    build_system_prompt, ChunkCallback, LlmError, LlmProvider, LlmResponse, TokenMetrics,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<OpenAIMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: OpenAIContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OpenAIContent {
    Text(String),
    Parts(Vec<OpenAIPart>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum OpenAIPart {
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

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    async fn send_with_image(
        &self,
        instruction: &str,
        image_base64: &str,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(LlmResponse, TokenMetrics), LlmError> {
        let start = Instant::now();
        let system_prompt = build_system_prompt(screen_width, screen_height);

        let request = OpenAIRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: OpenAIContent::Text(system_prompt),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: OpenAIContent::Parts(vec![
                        OpenAIPart::ImageUrl {
                            image_url: ImageUrl {
                                url: format!("data:image/png;base64,{}", image_base64),
                            },
                        },
                        OpenAIPart::Text {
                            text: format!(
                                "User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.",
                                instruction
                            ),
                        },
                    ]),
                },
            ],
            stream: true,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
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
        let mut full_response = String::new();
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

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
        }

        let metrics = TokenMetrics {
            input_tokens,
            output_tokens,
            total_duration: start.elapsed(),
        };

        Ok((LlmResponse::Text(full_response), metrics))
    }

    fn name(&self) -> &str {
        "openai"
    }
}
