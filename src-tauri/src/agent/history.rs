#![allow(dead_code)]

use super::action::Action;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

// ===== Session History (for export/logging) =====


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEntry {
    pub timestamp: DateTime<Utc>,
    pub iteration: u32,
    pub action_type: String,
    pub action_details: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_base64: Option<Arc<String>>,
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
            return serde_json::to_value(self).unwrap_or_default();
        }
        // Build JSON without cloning the entire history just to strip screenshots.
        // Each screenshot_base64 can be 1-2MB, so avoiding the clone is significant.
        let entries: Vec<serde_json::Value> = self.entries.iter().map(|entry| {
            let mut obj = serde_json::Map::new();
            obj.insert("timestamp".into(), serde_json::to_value(&entry.timestamp).unwrap_or_default());
            obj.insert("iteration".into(), serde_json::Value::Number(entry.iteration.into()));
            obj.insert("action_type".into(), serde_json::Value::String(entry.action_type.clone()));
            obj.insert("action_details".into(), entry.action_details.clone());
            obj.insert("llm_response".into(), serde_json::Value::String(entry.llm_response.clone()));
            obj.insert("success".into(), serde_json::Value::Bool(entry.success));
            if let Some(ref msg) = entry.error_message {
                obj.insert("error_message".into(), serde_json::Value::String(msg.clone()));
            }
            if let Some(ref msg) = entry.result_message {
                obj.insert("result_message".into(), serde_json::Value::String(msg.clone()));
            }
            serde_json::Value::Object(obj)
        }).collect();

        let mut obj = serde_json::Map::new();
        obj.insert("session_id".into(), serde_json::Value::String(self.session_id.clone()));
        obj.insert("instruction".into(), serde_json::Value::String(self.instruction.clone()));
        obj.insert("started_at".into(), serde_json::to_value(&self.started_at).unwrap_or_default());
        if let Some(ref ended) = self.ended_at {
            obj.insert("ended_at".into(), serde_json::to_value(ended).unwrap_or_default());
        }
        obj.insert("entries".into(), serde_json::Value::Array(entries));
        obj.insert("metrics".into(), serde_json::to_value(&self.metrics).unwrap_or_default());
        obj.insert("final_status".into(), serde_json::Value::String(self.final_status.clone()));
        serde_json::Value::Object(obj)
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

// ===== Undo History (for action reversal) =====

/// Record of an executed action with its reversibility information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// The action that was executed
    pub action: Action,
    /// When the action was executed
    pub timestamp: DateTime<Utc>,
    /// Whether the action executed successfully
    pub success: bool,
    /// Whether this action can be reversed
    pub reversible: bool,
    /// The action that would reverse this one, if any
    pub reverse_action: Option<Action>,
    /// Human-readable description of the action
    pub description: String,
}

impl ActionRecord {
    pub fn new(action: Action, success: bool) -> Self {
        let reversible = action.is_reversible();
        let reverse_action = action.create_reverse();
        let description = action.describe();

        Self {
            action,
            timestamp: Utc::now(),
            success,
            reversible,
            reverse_action,
            description,
        }
    }
}

/// History of executed actions with undo capability
#[derive(Debug, Clone)]
pub struct ActionHistory {
    records: VecDeque<ActionRecord>,
    max_size: usize,
}

impl Default for ActionHistory {
    fn default() -> Self {
        Self::new(50)
    }
}

impl ActionHistory {
    /// Create a new action history with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Add a new action record to the history
    pub fn push(&mut self, record: ActionRecord) {
        // Only track successful actions
        if !record.success {
            return;
        }

        if self.records.len() >= self.max_size {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// Remove and return the last action record
    pub fn pop_last(&mut self) -> Option<ActionRecord> {
        self.records.pop_back()
    }

    /// Get a reference to the last action record without removing it
    pub fn get_last(&self) -> Option<&ActionRecord> {
        self.records.back()
    }

    /// Check if there is an undoable action in the history
    pub fn can_undo(&self) -> bool {
        self.records
            .back()
            .map(|r| r.reversible && r.reverse_action.is_some())
            .unwrap_or(false)
    }

    /// Get the description of the last undoable action
    pub fn get_last_undoable_description(&self) -> Option<String> {
        self.records.back().and_then(|r| {
            if r.reversible {
                Some(r.description.clone())
            } else {
                None
            }
        })
    }

    /// Clear all history records
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Get the number of records in the history
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if the history is empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get all records for display (most recent first)
    pub fn get_recent(&self, count: usize) -> Vec<&ActionRecord> {
        self.records.iter().rev().take(count).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_history_basic() {
        let mut history = ActionHistory::new(10);
        assert!(history.is_empty());
        assert!(!history.can_undo());

        // Add a scroll action (reversible)
        let scroll_action = Action::Scroll {
            x: 100,
            y: 100,
            direction: "down".to_string(),
            amount: 3,
        };
        let record = ActionRecord::new(scroll_action, true);
        history.push(record);

        assert_eq!(history.len(), 1);
        assert!(history.can_undo());
    }

    #[test]
    fn test_action_history_max_size() {
        let mut history = ActionHistory::new(3);

        for i in 0..5 {
            let action = Action::Scroll {
                x: i,
                y: i,
                direction: "down".to_string(),
                amount: 1,
            };
            history.push(ActionRecord::new(action, true));
        }

        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_non_reversible_action() {
        let mut history = ActionHistory::new(10);

        // Click is not reversible
        let click_action = Action::Click {
            x: 100,
            y: 100,
            button: "left".to_string(),
        };
        let record = ActionRecord::new(click_action, true);
        history.push(record);

        assert_eq!(history.len(), 1);
        assert!(!history.can_undo()); // Can't undo a click
    }
}
