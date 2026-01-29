use super::provider::{build_system_prompt, history_to_messages, ChunkCallback, LlmError, LlmProvider, TokenMetrics};
use crate::agent::conversation::ConversationHistory;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct OllamaProvider {
    client: Client,
    host: String,
    model: String,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    images: Vec<String>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaStreamResponse {
    response: Option<String>,
    done: bool,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
}

impl OllamaProvider {
    pub fn new(host: String, model: String) -> Self {
        Self {
            client: Client::new(),
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
    ) -> Result<(String, TokenMetrics), LlmError> {
        let start = Instant::now();
        let system_prompt = build_system_prompt(screen_width, screen_height);

        // Build prompt from conversation history
        // Ollama uses a simple prompt format, so we concatenate messages
        let mut prompt = format!("{}\n\n", system_prompt);
        let mut images = Vec::new();

        for (role, text, image_base64) in history_to_messages(history) {
            let role_label = match role.as_str() {
                "user" => "User",
                "assistant" => "Assistant",
                _ => "System",
            };

            if let Some(img_data) = image_base64 {
                images.push(img_data);
                prompt.push_str(&format!(
                    "{}: [Screenshot attached]\n{}\n\nAnalyze the screenshot and respond with a single JSON action.\n\n",
                    role_label, text
                ));
            } else {
                prompt.push_str(&format!("{}: {}\n\n", role_label, text));
            }
        }

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            images,
            stream: true,
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.host))
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

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }

                if let Ok(parsed) = serde_json::from_str::<OllamaStreamResponse>(line) {
                    if let Some(response_text) = parsed.response {
                        full_response.push_str(&response_text);
                        on_chunk(&response_text);
                    }

                    if parsed.done {
                        output_tokens = parsed.eval_count.unwrap_or(0);
                        input_tokens = parsed.prompt_eval_count.unwrap_or(0);
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

    fn name(&self) -> &str {
        "ollama"
    }
}
