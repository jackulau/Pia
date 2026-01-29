use super::action::Action;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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
