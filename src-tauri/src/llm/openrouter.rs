use super::provider::{
    build_system_prompt, build_system_prompt_with_context, history_to_messages, ChunkCallback,
    LlmError, LlmProvider, LlmResponse, TokenMetrics,
};
use super::sse::{append_bytes_to_buffer, process_sse_buffer};
use serde_json::Value;
use crate::agent::conversation::ConversationHistory;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub struct OpenRouterProvider {
    client: Client,
    api_key: String,
    model: String,
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct OpenRouterRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<OpenRouterMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct OpenRouterMessage {
    role: String,
    content: OpenRouterContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OpenRouterContent {
    Text(String),
    Parts(Vec<OpenRouterPart>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum OpenRouterPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: String, temperature: Option<f32>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            temperature,
        }
    }

    pub fn with_timeouts(api_key: String, model: String, temperature: Option<f32>, connect_timeout: Duration, response_timeout: Duration) -> Self {
        let client = Client::builder()
            .connect_timeout(connect_timeout)
            .timeout(response_timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            api_key,
            model,
            temperature,
        }
    }

    pub fn with_temperature(mut self, temperature: Option<f32>) -> Self {
        self.temperature = temperature;
        self
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn send_with_history(
        &self,
        history: &ConversationHistory,
        screen_width: u32,
        screen_height: u32,
        on_chunk: ChunkCallback,
    ) -> Result<(LlmResponse, TokenMetrics), LlmError> {
        let start = Instant::now();
        let instruction = history.original_instruction().map(|s| s.to_string());
        let system_prompt = build_system_prompt_with_context(
            screen_width,
            screen_height,
            instruction.as_deref(),
            history.iteration,
            history.max_iterations,
        );

        // Build messages from conversation history
        let mut messages = vec![OpenRouterMessage {
            role: "system".to_string(),
            content: OpenRouterContent::Text(system_prompt),
        }];

        for (role, text, image_base64) in history_to_messages(history) {
            let content = if let Some(img_data) = image_base64 {
                OpenRouterContent::Parts(vec![
                    OpenRouterPart::ImageUrl {
                        image_url: ImageUrl {
                            url: format!("data:image/png;base64,{}", &*img_data),
                        },
                    },
                    OpenRouterPart::Text {
                        text: format!(
                            "User instruction: {}\n\nAnalyze the screenshot and respond with a single JSON action.",
                            text
                        ),
                    },
                ])
            } else {
                OpenRouterContent::Text(text)
            };

            messages.push(OpenRouterMessage { role, content });
        }

        let request = OpenRouterRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages,
            stream: true,
            temperature: self.temperature,
        };

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/jackulau/Pia")
            .header("X-Title", "Pia Computer Use Agent")
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

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            append_bytes_to_buffer(&mut buffer, &chunk);

            let result = process_sse_buffer(&mut buffer, &mut full_response, &*on_chunk);
            if let Some(t) = result.input_tokens {
                input_tokens = t;
            }
            if let Some(t) = result.output_tokens {
                output_tokens = t;
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
            .get("https://openrouter.ai/api/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let response = self
            .client
            .get("https://openrouter.ai/api/v1/models")
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
        "openrouter"
    }
}
