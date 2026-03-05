#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Maximum number of messages to keep in history to prevent unbounded memory growth.
/// Each message includes a screenshot (~1-2MB base64), so we limit to recent context.
const MAX_HISTORY_LENGTH: usize = 20;

/// Represents different types of messages in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// User message containing instruction and optional screenshot
    User {
        instruction: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        screenshot_base64: Option<Arc<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        screen_width: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        screen_height: Option<u32>,
    },
    /// Assistant response with the action JSON
    Assistant { content: String },
    /// Result of executing a tool/action
    ToolResult {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}

/// Manages conversation history for the agent loop.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationHistory {
    messages: VecDeque<Message>,
    /// The original user instruction for this task
    #[serde(skip_serializing_if = "Option::is_none")]
    original_instruction: Option<String>,
    /// Current iteration number (set by the agent loop for progress context)
    #[serde(skip)]
    pub iteration: Option<u32>,
    /// Maximum iterations allowed (set by the agent loop for progress context)
    #[serde(skip)]
    pub max_iterations: Option<u32>,
}

impl ConversationHistory {
    /// Creates a new empty conversation history.
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            original_instruction: None,
            iteration: None,
            max_iterations: None,
        }
    }

    /// Sets the original instruction for this conversation.
    pub fn set_original_instruction(&mut self, instruction: String) {
        self.original_instruction = Some(instruction);
    }

    /// Gets the original instruction if set.
    pub fn original_instruction(&self) -> Option<&str> {
        self.original_instruction.as_deref()
    }

    /// Adds a message to the conversation history.
    /// Automatically truncates if history exceeds MAX_HISTORY_LENGTH.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push_back(message);
        self.truncate_to_max();
    }

    /// Adds a user message with instruction and screenshot.
    pub fn add_user_message(
        &mut self,
        instruction: &str,
        screenshot_base64: Option<Arc<String>>,
        screen_width: Option<u32>,
        screen_height: Option<u32>,
    ) {
        self.add_message(Message::User {
            instruction: instruction.to_string(),
            screenshot_base64,
            screen_width,
            screen_height,
        });
    }

    /// Adds an assistant response message.
    pub fn add_assistant_message(&mut self, content: &str) {
        self.add_message(Message::Assistant {
            content: content.to_string(),
        });
    }

    /// Adds a tool result message.
    pub fn add_tool_result(
        &mut self,
        success: bool,
        message: Option<String>,
        error: Option<String>,
    ) {
        self.add_message(Message::ToolResult {
            success,
            message,
            error,
        });
    }

    /// Returns all messages as a contiguous slice.
    pub fn get_messages(&mut self) -> &[Message] {
        self.messages.make_contiguous();
        self.messages.as_slices().0
    }

    /// Returns an iterator over all messages.
    pub fn messages(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }

    /// Returns the number of messages in history.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Returns true if history is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clears all messages from history.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.original_instruction = None;
        self.iteration = None;
        self.max_iterations = None;
    }

    /// Truncates history to MAX_HISTORY_LENGTH, keeping most recent messages.
    /// Always preserves the first message (original instruction) if possible.
    fn truncate_to_max(&mut self) {
        if self.messages.len() > MAX_HISTORY_LENGTH {
            // Keep first message (original context) and most recent messages
            let first = self.messages.pop_front().unwrap();
            let excess = self.messages.len() - (MAX_HISTORY_LENGTH - 1);
            drop(self.messages.drain(..excess));
            self.messages.push_front(first);
        }
    }

    /// Gets the last assistant message if available.
    pub fn last_assistant_message(&self) -> Option<&str> {
        self.messages.iter().rev().find_map(|m| {
            if let Message::Assistant { content } = m {
                Some(content.as_str())
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_conversation() {
        let conv = ConversationHistory::new();
        assert!(conv.is_empty());
        assert_eq!(conv.len(), 0);
    }

    #[test]
    fn test_add_user_message() {
        let mut conv = ConversationHistory::new();
        conv.add_user_message(
            "Click the button",
            Some(Arc::new("base64data".to_string())),
            Some(1920),
            Some(1080),
        );

        assert_eq!(conv.len(), 1);
        match &conv.get_messages()[0] {
            Message::User {
                instruction,
                screenshot_base64,
                screen_width,
                screen_height,
            } => {
                assert_eq!(instruction, "Click the button");
                assert_eq!(
                    screenshot_base64.as_ref().map(|s| s.as_str()),
                    Some("base64data")
                );
                assert_eq!(*screen_width, Some(1920));
                assert_eq!(*screen_height, Some(1080));
            }
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn test_add_assistant_message() {
        let mut conv = ConversationHistory::new();
        conv.add_assistant_message(r#"{"action": "click", "x": 100, "y": 200}"#);

        assert_eq!(conv.len(), 1);
        match &conv.get_messages()[0] {
            Message::Assistant { content } => {
                assert!(content.contains("click"));
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_add_tool_result() {
        let mut conv = ConversationHistory::new();
        conv.add_tool_result(true, Some("Clicked successfully".to_string()), None);

        assert_eq!(conv.len(), 1);
        match &conv.get_messages()[0] {
            Message::ToolResult {
                success,
                message,
                error,
            } => {
                assert!(*success);
                assert_eq!(message.as_deref(), Some("Clicked successfully"));
                assert!(error.is_none());
            }
            _ => panic!("Expected ToolResult message"),
        }
    }

    #[test]
    fn test_clear() {
        let mut conv = ConversationHistory::new();
        conv.set_original_instruction("Test instruction".to_string());
        conv.add_user_message("Test", None, None, None);
        conv.add_assistant_message("Response");

        assert_eq!(conv.len(), 2);
        conv.clear();
        assert!(conv.is_empty());
        assert!(conv.original_instruction().is_none());
    }

    #[test]
    fn test_truncation() {
        let mut conv = ConversationHistory::new();

        // Add more messages than MAX_HISTORY_LENGTH
        for i in 0..25 {
            conv.add_user_message(&format!("Message {}", i), None, None, None);
        }

        // Should be truncated to MAX_HISTORY_LENGTH
        assert!(conv.len() <= 20);

        // First message should still be preserved
        match &conv.get_messages()[0] {
            Message::User { instruction, .. } => {
                assert_eq!(instruction, "Message 0");
            }
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn test_original_instruction() {
        let mut conv = ConversationHistory::new();
        assert!(conv.original_instruction().is_none());

        conv.set_original_instruction("Open browser".to_string());
        assert_eq!(conv.original_instruction(), Some("Open browser"));
    }

    #[test]
    fn test_last_assistant_message() {
        let mut conv = ConversationHistory::new();
        assert!(conv.last_assistant_message().is_none());

        conv.add_user_message("Test", None, None, None);
        conv.add_assistant_message("First response");
        conv.add_user_message("Test 2", None, None, None);
        conv.add_assistant_message("Second response");

        assert_eq!(conv.last_assistant_message(), Some("Second response"));
    }

    #[test]
    fn test_serialization() {
        let mut conv = ConversationHistory::new();
        conv.set_original_instruction("Test".to_string());
        conv.add_user_message("Click button", None, None, None);
        conv.add_assistant_message(r#"{"action": "click"}"#);

        let json = serde_json::to_string(&conv).unwrap();
        let deserialized: ConversationHistory = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized.original_instruction(), Some("Test"));
    }
}
