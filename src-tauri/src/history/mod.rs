use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

const MAX_HISTORY_ENTRIES: usize = 50;

#[derive(Error, Debug)]
pub enum HistoryError {
    #[error("Failed to read history file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse history: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("History directory not found")]
    NoDirFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub instruction: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstructionHistory {
    entries: Vec<HistoryEntry>,
}

impl InstructionHistory {
    pub fn history_path() -> Result<PathBuf, HistoryError> {
        let config_dir = dirs::config_dir().ok_or(HistoryError::NoDirFound)?;
        Ok(config_dir.join("pia").join("history.json"))
    }

    pub fn load() -> Result<Self, HistoryError> {
        let path = Self::history_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        let history: InstructionHistory = serde_json::from_str(&content)?;
        Ok(history)
    }

    pub fn save(&self) -> Result<(), HistoryError> {
        let path = Self::history_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn add(&mut self, instruction: String, success: bool) {
        // Check if this instruction already exists at the top
        if let Some(first) = self.entries.first() {
            if first.instruction == instruction {
                // Update the existing entry instead of adding a duplicate
                self.entries[0] = HistoryEntry {
                    instruction,
                    timestamp: Utc::now(),
                    success,
                };
                return;
            }
        }

        // Remove any existing entry with the same instruction
        self.entries.retain(|e| e.instruction != instruction);

        // Add new entry at the front
        self.entries.insert(
            0,
            HistoryEntry {
                instruction,
                timestamp: Utc::now(),
                success,
            },
        );

        // Trim to max size
        if self.entries.len() > MAX_HISTORY_ENTRIES {
            self.entries.truncate(MAX_HISTORY_ENTRIES);
        }
    }

    pub fn get_all(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.entries.len() {
            self.entries.remove(index);
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
