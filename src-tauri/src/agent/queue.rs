#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueItemStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueFailureMode {
    Stop,
    Continue,
}

impl Default for QueueFailureMode {
    fn default() -> Self {
        Self::Stop
    }
}

impl From<&str> for QueueFailureMode {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "continue" => Self::Continue,
            _ => Self::Stop,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedInstruction {
    pub id: String,
    pub instruction: String,
    pub status: QueueItemStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

impl QueuedInstruction {
    pub fn new(instruction: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            instruction,
            status: QueueItemStatus::Pending,
            result: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionQueue {
    pub items: VecDeque<QueuedInstruction>,
    pub current_index: usize,
    pub is_processing: bool,
    pub failure_mode: QueueFailureMode,
    #[serde(default)]
    pending_count: usize,
    #[serde(default)]
    completed_count: usize,
}

impl Default for InstructionQueue {
    fn default() -> Self {
        Self {
            items: VecDeque::new(),
            current_index: 0,
            is_processing: false,
            failure_mode: QueueFailureMode::default(),
            pending_count: 0,
            completed_count: 0,
        }
    }
}

impl InstructionQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, instruction: String) -> String {
        let item = QueuedInstruction::new(instruction);
        let id = item.id.clone();
        self.items.push_back(item);
        self.pending_count += 1;
        id
    }

    pub fn add_multiple(&mut self, instructions: Vec<String>) -> Vec<String> {
        instructions.into_iter().map(|i| self.add(i)).collect()
    }

    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(pos) = self.items.iter().position(|item| item.id == id) {
            // Don't allow removing currently running item
            if pos == self.current_index && self.is_processing {
                return false;
            }
            let status = self.items[pos].status;
            self.items.remove(pos);
            match status {
                QueueItemStatus::Pending => self.pending_count = self.pending_count.saturating_sub(1),
                QueueItemStatus::Completed => self.completed_count = self.completed_count.saturating_sub(1),
                _ => {}
            }
            // Adjust current_index if needed
            if pos < self.current_index && self.current_index > 0 {
                self.current_index -= 1;
            }
            true
        } else {
            false
        }
    }

    pub fn get_next(&mut self) -> Option<&QueuedInstruction> {
        // Find next pending item starting from current_index
        for i in self.current_index..self.items.len() {
            if self.items[i].status == QueueItemStatus::Pending {
                self.current_index = i;
                return self.items.get(i);
            }
        }
        None
    }

    pub fn get_current(&self) -> Option<&QueuedInstruction> {
        self.items.get(self.current_index)
    }

    pub fn mark_current_running(&mut self) {
        if let Some(item) = self.items.get_mut(self.current_index) {
            if item.status == QueueItemStatus::Pending {
                self.pending_count = self.pending_count.saturating_sub(1);
            }
            item.status = QueueItemStatus::Running;
        }
    }

    pub fn mark_current_completed(&mut self, result: Option<String>) {
        if let Some(item) = self.items.get_mut(self.current_index) {
            item.status = QueueItemStatus::Completed;
            item.result = result;
            self.completed_count += 1;
        }
    }

    pub fn mark_current_failed(&mut self, error: String) {
        if let Some(item) = self.items.get_mut(self.current_index) {
            item.status = QueueItemStatus::Failed;
            item.error = Some(error);
        }
    }

    pub fn advance(&mut self) -> bool {
        if self.current_index + 1 < self.items.len() {
            self.current_index += 1;
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current_index = 0;
        self.is_processing = false;
        self.pending_count = 0;
        self.completed_count = 0;
    }

    pub fn clear_pending(&mut self) {
        self.items.retain(|item| item.status != QueueItemStatus::Pending);
        self.pending_count = 0;
    }

    pub fn reorder(&mut self, ids: Vec<String>) -> bool {
        // Only reorder pending items
        let pending: Vec<_> = self.items.iter()
            .filter(|item| item.status == QueueItemStatus::Pending)
            .cloned()
            .collect();

        // Check that all provided IDs match pending items
        if pending.len() != ids.len() {
            return false;
        }

        let mut new_pending: Vec<QueuedInstruction> = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Some(item) = pending.iter().find(|i| &i.id == id) {
                new_pending.push(item.clone());
            } else {
                return false;
            }
        }

        // Rebuild queue: non-pending items stay in place, pending items get reordered
        let mut new_items: VecDeque<QueuedInstruction> = VecDeque::new();
        let mut pending_iter = new_pending.into_iter();

        for item in &self.items {
            if item.status == QueueItemStatus::Pending {
                if let Some(new_item) = pending_iter.next() {
                    new_items.push_back(new_item);
                }
            } else {
                new_items.push_back(item.clone());
            }
        }

        self.items = new_items;
        true
    }

    pub fn get_all(&self) -> Vec<QueuedInstruction> {
        self.items.iter().cloned().collect()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_count
    }

    pub fn completed_count(&self) -> usize {
        self.completed_count
    }

    pub fn total_count(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_pending(&self) -> bool {
        self.pending_count > 0
    }
}

#[derive(Clone)]
pub struct QueueManager {
    queue: Arc<RwLock<InstructionQueue>>,
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(InstructionQueue::new())),
        }
    }

    pub async fn add(&self, instruction: String) -> String {
        self.queue.write().await.add(instruction)
    }

    pub async fn add_multiple(&self, instructions: Vec<String>) -> Vec<String> {
        self.queue.write().await.add_multiple(instructions)
    }

    pub async fn remove(&self, id: &str) -> bool {
        self.queue.write().await.remove(id)
    }

    pub async fn get_next(&self) -> Option<QueuedInstruction> {
        self.queue.write().await.get_next().cloned()
    }

    pub async fn get_current(&self) -> Option<QueuedInstruction> {
        self.queue.read().await.get_current().cloned()
    }

    pub async fn mark_current_running(&self) {
        self.queue.write().await.mark_current_running();
    }

    pub async fn mark_current_completed(&self, result: Option<String>) {
        self.queue.write().await.mark_current_completed(result);
    }

    pub async fn mark_current_failed(&self, error: String) {
        self.queue.write().await.mark_current_failed(error);
    }

    pub async fn advance(&self) -> bool {
        self.queue.write().await.advance()
    }

    pub async fn clear(&self) {
        self.queue.write().await.clear();
    }

    pub async fn clear_pending(&self) {
        self.queue.write().await.clear_pending();
    }

    pub async fn reorder(&self, ids: Vec<String>) -> bool {
        self.queue.write().await.reorder(ids)
    }

    pub async fn get_all(&self) -> Vec<QueuedInstruction> {
        self.queue.read().await.get_all()
    }

    pub async fn get_state(&self) -> InstructionQueue {
        self.queue.read().await.clone()
    }

    pub async fn set_processing(&self, processing: bool) {
        self.queue.write().await.is_processing = processing;
    }

    pub async fn is_processing(&self) -> bool {
        self.queue.read().await.is_processing
    }

    pub async fn set_failure_mode(&self, mode: QueueFailureMode) {
        self.queue.write().await.failure_mode = mode;
    }

    pub async fn get_failure_mode(&self) -> QueueFailureMode {
        self.queue.read().await.failure_mode
    }

    pub async fn has_pending(&self) -> bool {
        self.queue.read().await.has_pending()
    }

    pub async fn pending_count(&self) -> usize {
        self.queue.read().await.pending_count()
    }

    pub async fn completed_count(&self) -> usize {
        self.queue.read().await.completed_count()
    }

    pub async fn total_count(&self) -> usize {
        self.queue.read().await.total_count()
    }

    pub async fn current_index(&self) -> usize {
        self.queue.read().await.current_index
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_instruction() {
        let mut queue = InstructionQueue::new();
        let id = queue.add("Test instruction".to_string());
        assert!(!id.is_empty());
        assert_eq!(queue.total_count(), 1);
        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn test_add_multiple() {
        let mut queue = InstructionQueue::new();
        let ids = queue.add_multiple(vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ]);
        assert_eq!(ids.len(), 3);
        assert_eq!(queue.total_count(), 3);
    }

    #[test]
    fn test_remove() {
        let mut queue = InstructionQueue::new();
        let id = queue.add("Test".to_string());
        assert!(queue.remove(&id));
        assert_eq!(queue.total_count(), 0);
    }

    #[test]
    fn test_get_next() {
        let mut queue = InstructionQueue::new();
        queue.add("First".to_string());
        queue.add("Second".to_string());

        let next = queue.get_next();
        assert!(next.is_some());
        assert_eq!(next.unwrap().instruction, "First");
    }

    #[test]
    fn test_mark_completed_and_advance() {
        let mut queue = InstructionQueue::new();
        queue.add("First".to_string());
        queue.add("Second".to_string());

        queue.get_next();
        queue.mark_current_running();
        queue.mark_current_completed(Some("Done".to_string()));

        assert_eq!(queue.completed_count(), 1);

        assert!(queue.advance());
        let next = queue.get_current();
        assert!(next.is_some());
        assert_eq!(next.unwrap().instruction, "Second");
    }

    #[test]
    fn test_clear() {
        let mut queue = InstructionQueue::new();
        queue.add("First".to_string());
        queue.add("Second".to_string());
        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_reorder() {
        let mut queue = InstructionQueue::new();
        let id1 = queue.add("First".to_string());
        let id2 = queue.add("Second".to_string());
        let id3 = queue.add("Third".to_string());

        // Reorder to: Third, First, Second
        assert!(queue.reorder(vec![id3.clone(), id1.clone(), id2.clone()]));

        let items = queue.get_all();
        assert_eq!(items[0].instruction, "Third");
        assert_eq!(items[1].instruction, "First");
        assert_eq!(items[2].instruction, "Second");
    }
}
