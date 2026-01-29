use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEntry {
    pub timestamp: DateTime<Utc>,
    pub iteration: u32,
    pub action_type: String,
    pub action_details: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_base64: Option<String>,
    pub llm_response: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_iterations: u32,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHistory {
    pub session_id: String,
    pub instruction: String,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    pub entries: Vec<ActionEntry>,
    pub metrics: SessionMetrics,
    pub final_status: String,
}

impl SessionHistory {
    pub fn new(instruction: String) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            instruction,
            started_at: Utc::now(),
            ended_at: None,
            entries: Vec::new(),
            metrics: SessionMetrics {
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_iterations: 0,
                duration_seconds: 0.0,
            },
            final_status: "running".to_string(),
        }
    }

    pub fn add_entry(&mut self, entry: ActionEntry) {
        self.metrics.total_iterations = entry.iteration;
        self.entries.push(entry);
    }

    pub fn update_metrics(&mut self, input_tokens: u64, output_tokens: u64) {
        self.metrics.total_input_tokens += input_tokens;
        self.metrics.total_output_tokens += output_tokens;
    }

    pub fn complete(&mut self, status: &str) {
        self.ended_at = Some(Utc::now());
        self.final_status = status.to_string();
        if let Some(ended) = self.ended_at {
            self.metrics.duration_seconds = (ended - self.started_at).num_milliseconds() as f64 / 1000.0;
        }
    }

    pub fn to_json(&self, include_screenshots: bool) -> serde_json::Value {
        if include_screenshots {
            serde_json::to_value(self).unwrap_or_default()
        } else {
            let mut history = self.clone();
            for entry in &mut history.entries {
                entry.screenshot_base64 = None;
            }
            serde_json::to_value(history).unwrap_or_default()
        }
    }

    pub fn to_text(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("Session: {}\n", self.started_at.format("%Y-%m-%d %H:%M:%S UTC")));
        output.push_str(&format!("Instruction: \"{}\"\n", self.instruction));
        output.push_str(&format!("Status: {}\n", self.final_status));
        output.push('\n');

        for entry in &self.entries {
            output.push_str(&format!(
                "[{}] {} - {}\n",
                entry.iteration,
                entry.timestamp.format("%H:%M:%S"),
                entry.action_type
            ));

            if let Some(details) = entry.action_details.as_object() {
                let detail_str = details
                    .iter()
                    .filter(|(k, _)| *k != "action")
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ");
                if !detail_str.is_empty() {
                    output.push_str(&format!("    Details: {}\n", detail_str));
                }
            }

            // Truncate LLM response for readability
            let llm_preview = if entry.llm_response.len() > 200 {
                format!("{}...", &entry.llm_response[..200])
            } else {
                entry.llm_response.clone()
            };
            output.push_str(&format!("    LLM: \"{}\"\n", llm_preview.replace('\n', " ")));

            let result = if entry.success { "Success" } else { "Failed" };
            output.push_str(&format!("    Result: {}", result));

            if let Some(msg) = &entry.result_message {
                output.push_str(&format!(" - {}", msg));
            }
            if let Some(err) = &entry.error_message {
                output.push_str(&format!(" ({})", err));
            }
            output.push_str("\n\n");
        }

        output.push_str("--- Metrics ---\n");
        output.push_str(&format!("Total Iterations: {}\n", self.metrics.total_iterations));
        output.push_str(&format!("Input Tokens: {}\n", self.metrics.total_input_tokens));
        output.push_str(&format!("Output Tokens: {}\n", self.metrics.total_output_tokens));
        output.push_str(&format!("Duration: {:.2}s\n", self.metrics.duration_seconds));

        if let Some(ended) = self.ended_at {
            output.push_str(&format!("Ended: {}\n", ended.format("%Y-%m-%d %H:%M:%S UTC")));
        }

        output
    }
}

#[derive(Clone)]
pub struct HistoryManager {
    current_session: Arc<RwLock<Option<SessionHistory>>>,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            current_session: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start_session(&self, instruction: String) {
        let mut session = self.current_session.write().await;
        *session = Some(SessionHistory::new(instruction));
    }

    pub async fn add_entry(&self, entry: ActionEntry) {
        let mut session = self.current_session.write().await;
        if let Some(ref mut s) = *session {
            s.add_entry(entry);
        }
    }

    pub async fn update_metrics(&self, input_tokens: u64, output_tokens: u64) {
        let mut session = self.current_session.write().await;
        if let Some(ref mut s) = *session {
            s.update_metrics(input_tokens, output_tokens);
        }
    }

    pub async fn complete_session(&self, status: &str) {
        let mut session = self.current_session.write().await;
        if let Some(ref mut s) = *session {
            s.complete(status);
        }
    }

    pub async fn get_session(&self) -> Option<SessionHistory> {
        let session = self.current_session.read().await;
        session.clone()
    }

    pub async fn get_entry_count(&self) -> usize {
        let session = self.current_session.read().await;
        session.as_ref().map(|s| s.entries.len()).unwrap_or(0)
    }

    pub async fn clear(&self) {
        let mut session = self.current_session.write().await;
        *session = None;
    }

    pub async fn export_json(&self, include_screenshots: bool) -> Option<String> {
        let session = self.current_session.read().await;
        session.as_ref().map(|s| s.to_json(include_screenshots).to_string())
    }

    pub async fn export_text(&self) -> Option<String> {
        let session = self.current_session.read().await;
        session.as_ref().map(|s| s.to_text())
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}
