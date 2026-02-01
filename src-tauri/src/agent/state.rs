use chrono::Utc;
use super::history::HistoryManager;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationResponse {
    Confirmed,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Recording,
    Paused,
    AwaitingConfirmation,
    Retrying,
    Completed,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Normal,
    Recording,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedAction {
    pub action: String,
    pub reasoning: Option<String>,
    pub timestamp: u64,
    pub iteration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHistoryEntry {
    pub action: String,
    pub timestamp: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub status: AgentStatus,
    pub instruction: Option<String>,
    pub iteration: u32,
    pub max_iterations: u32,
    pub last_action: Option<String>,
    pub last_reasoning: Option<String>,
    pub last_error: Option<String>,
    pub pending_action: Option<String>,
    pub tokens_per_second: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub action_history: Vec<ActionHistoryEntry>,
    pub retry_count: u32,
    pub consecutive_errors: u32,
    pub queue_index: usize,
    pub queue_total: usize,
    pub queue_active: bool,
    pub preview_mode: bool,
    pub last_screenshot: Option<String>,
    pub kill_switch_triggered: bool,
    pub execution_mode: ExecutionMode,
    pub recorded_actions: Vec<RecordedAction>,
    pub last_retry_count: u32,
    pub total_retries: u32,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            status: AgentStatus::Idle,
            instruction: None,
            iteration: 0,
            max_iterations: 50,
            last_action: None,
            last_reasoning: None,
            last_error: None,
            pending_action: None,
            tokens_per_second: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            action_history: Vec::new(),
            retry_count: 0,
            consecutive_errors: 0,
            queue_index: 0,
            queue_total: 0,
            queue_active: false,
            preview_mode: false,
            last_screenshot: None,
            kill_switch_triggered: false,
            execution_mode: ExecutionMode::Normal,
            recorded_actions: Vec::new(),
            last_retry_count: 0,
            total_retries: 0,
        }
    }
}

/// Atomic metrics for frequently updated values, avoiding RwLock contention
struct AtomicMetrics {
    iteration: AtomicU32,
    max_iterations: AtomicU32,
    total_input_tokens: AtomicU64,
    total_output_tokens: AtomicU64,
    /// tokens_per_second stored as bits (use f64::to_bits/from_bits)
    tokens_per_second_bits: AtomicU64,
}

impl AtomicMetrics {
    fn new() -> Self {
        Self {
            iteration: AtomicU32::new(0),
            max_iterations: AtomicU32::new(50),
            total_input_tokens: AtomicU64::new(0),
            total_output_tokens: AtomicU64::new(0),
            tokens_per_second_bits: AtomicU64::new(0.0_f64.to_bits()),
        }
    }

    fn set_tokens_per_second(&self, value: f64) {
        self.tokens_per_second_bits
            .store(value.to_bits(), Ordering::Release);
    }

    fn get_tokens_per_second(&self) -> f64 {
        f64::from_bits(self.tokens_per_second_bits.load(Ordering::Acquire))
    }

    fn reset(&self) {
        self.iteration.store(0, Ordering::Release);
        self.max_iterations.store(50, Ordering::Release);
        self.total_input_tokens.store(0, Ordering::Release);
        self.total_output_tokens.store(0, Ordering::Release);
        self.tokens_per_second_bits
            .store(0.0_f64.to_bits(), Ordering::Release);
    }
}

pub struct AgentStateManager {
    state: Arc<RwLock<AgentState>>,
    metrics: Arc<AtomicMetrics>,
    should_stop: Arc<AtomicBool>,
    should_pause: Arc<AtomicBool>,
    confirmation_tx: Arc<RwLock<Option<mpsc::Sender<ConfirmationResponse>>>>,
    confirmation_rx: Arc<RwLock<Option<mpsc::Receiver<ConfirmationResponse>>>>,
    history: HistoryManager,
    kill_switch_triggered: Arc<AtomicBool>,
}

impl Clone for AgentStateManager {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            metrics: Arc::clone(&self.metrics),
            should_stop: Arc::clone(&self.should_stop),
            should_pause: Arc::clone(&self.should_pause),
            confirmation_tx: Arc::clone(&self.confirmation_tx),
            confirmation_rx: Arc::clone(&self.confirmation_rx),
            history: self.history.clone(),
            kill_switch_triggered: Arc::clone(&self.kill_switch_triggered),
        }
    }
}

impl AgentStateManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);
        Self {
            state: Arc::new(RwLock::new(AgentState::default())),
            metrics: Arc::new(AtomicMetrics::new()),
            should_stop: Arc::new(AtomicBool::new(false)),
            should_pause: Arc::new(AtomicBool::new(false)),
            confirmation_tx: Arc::new(RwLock::new(Some(tx))),
            confirmation_rx: Arc::new(RwLock::new(Some(rx))),
            history: HistoryManager::new(),
            kill_switch_triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn history(&self) -> &HistoryManager {
        &self.history
    }

    /// Get the full state, merging atomic metrics with RwLock-protected fields.
    pub async fn get_state(&self) -> AgentState {
        let mut state = self.state.read().await.clone();
        // Override with atomic values for consistency
        state.iteration = self.metrics.iteration.load(Ordering::Acquire);
        state.max_iterations = self.metrics.max_iterations.load(Ordering::Acquire);
        state.tokens_per_second = self.metrics.get_tokens_per_second();
        state.total_input_tokens = self.metrics.total_input_tokens.load(Ordering::Acquire);
        state.total_output_tokens = self.metrics.total_output_tokens.load(Ordering::Acquire);
        state.kill_switch_triggered = self.kill_switch_triggered.load(Ordering::SeqCst);
        state
    }

    // --- Specific getters to avoid full state clones ---

    /// Get current iteration count without acquiring RwLock
    pub fn get_iteration(&self) -> u32 {
        self.metrics.iteration.load(Ordering::Acquire)
    }

    /// Get max iterations without acquiring RwLock
    pub fn get_max_iterations(&self) -> u32 {
        self.metrics.max_iterations.load(Ordering::Acquire)
    }

    /// Get token metrics without acquiring RwLock
    pub fn get_token_metrics(&self) -> (f64, u64, u64) {
        (
            self.metrics.get_tokens_per_second(),
            self.metrics.total_input_tokens.load(Ordering::Acquire),
            self.metrics.total_output_tokens.load(Ordering::Acquire),
        )
    }

    /// Get status (requires read lock, but minimal clone)
    pub async fn get_status(&self) -> AgentStatus {
        self.state.read().await.status
    }

    pub async fn set_status(&self, status: AgentStatus) {
        let mut state = self.state.write().await;
        state.status = status;
    }

    pub async fn start(&self, instruction: String, max_iterations: u32) {
        self.start_with_mode(instruction, max_iterations, ExecutionMode::Normal).await;
    }

    pub async fn start_recording(&self, instruction: String, max_iterations: u32) {
        self.start_with_mode(instruction, max_iterations, ExecutionMode::Recording).await;
    }

    pub async fn start_with_mode(&self, instruction: String, max_iterations: u32, mode: ExecutionMode) {
        // Reset atomic metrics first (no lock needed)
        self.metrics.iteration.store(0, Ordering::Release);
        self.metrics.max_iterations.store(max_iterations, Ordering::Release);
        self.metrics.total_input_tokens.store(0, Ordering::Release);
        self.metrics.total_output_tokens.store(0, Ordering::Release);
        self.metrics.set_tokens_per_second(0.0);
        self.should_stop.store(false, Ordering::SeqCst);

        // Now update the RwLock-protected state
        let mut state = self.state.write().await;
        state.status = match mode {
            ExecutionMode::Normal => AgentStatus::Running,
            ExecutionMode::Recording => AgentStatus::Recording,
        };
        state.instruction = Some(instruction.clone());
        state.iteration = 0;
        state.max_iterations = max_iterations;
        state.last_action = None;
        state.last_reasoning = None;
        state.last_error = None;
        state.tokens_per_second = 0.0;
        state.total_input_tokens = 0;
        state.total_output_tokens = 0;
        state.action_history.clear();
        state.retry_count = 0;
        state.consecutive_errors = 0;
        state.execution_mode = mode;
        state.recorded_actions = Vec::new();
        state.last_retry_count = 0;
        state.total_retries = 0;
        self.should_pause.store(false, Ordering::SeqCst);
        self.kill_switch_triggered.store(false, Ordering::SeqCst);
        // Start a new history session
        drop(state); // Release write lock before async call
        self.history.start_session(instruction).await;
    }

    pub async fn add_recorded_action(&self, action: String, reasoning: Option<String>) {
        let mut state = self.state.write().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let iteration = self.metrics.iteration.load(Ordering::Acquire);
        state.recorded_actions.push(RecordedAction {
            action,
            reasoning,
            timestamp,
            iteration,
        });
    }

    pub async fn get_recorded_actions(&self) -> Vec<RecordedAction> {
        self.state.read().await.recorded_actions.clone()
    }

    pub async fn clear_recorded_actions(&self) {
        let mut state = self.state.write().await;
        state.recorded_actions.clear();
    }

    pub async fn get_execution_mode(&self) -> ExecutionMode {
        self.state.read().await.execution_mode
    }

    /// Atomically increment iteration counter without acquiring RwLock
    pub fn increment_iteration(&self) -> u32 {
        self.metrics.iteration.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub async fn set_last_action(&self, action: String) {
        let mut state = self.state.write().await;
        state.last_action = Some(action.clone());
        state.action_history.push(ActionHistoryEntry {
            action,
            timestamp: Utc::now().to_rfc3339(),
            is_error: false,
        });
    }

    pub async fn set_last_reasoning(&self, reasoning: Option<String>) {
        let mut state = self.state.write().await;
        state.last_reasoning = reasoning;
    }

    pub async fn set_error(&self, error: String) {
        let mut state = self.state.write().await;
        state.status = AgentStatus::Error;
        state.last_error = Some(error.clone());
        state.action_history.push(ActionHistoryEntry {
            action: error,
            timestamp: Utc::now().to_rfc3339(),
            is_error: true,
        });
    }

    pub async fn set_last_screenshot(&self, screenshot: String) {
        let mut state = self.state.write().await;
        state.last_screenshot = Some(screenshot);
    }

    /// Update metrics atomically without acquiring RwLock
    pub fn update_metrics(&self, tokens_per_sec: f64, input_tokens: u64, output_tokens: u64) {
        self.metrics.set_tokens_per_second(tokens_per_sec);
        self.metrics.total_input_tokens.fetch_add(input_tokens, Ordering::AcqRel);
        self.metrics.total_output_tokens.fetch_add(output_tokens, Ordering::AcqRel);
    }

    pub async fn update_retry_stats(&self, retry_count: u32) {
        let mut state = self.state.write().await;
        state.last_retry_count = retry_count;
        state.total_retries += retry_count;
    }

    pub async fn complete(&self, message: Option<String>) {
        let mut state = self.state.write().await;
        state.status = AgentStatus::Completed;
        if let Some(msg) = message {
            state.last_action = Some(format!("Completed: {}", msg));
        }
    }

    pub fn request_stop(&self) {
        self.should_stop.store(true, Ordering::SeqCst);
    }

    pub fn trigger_kill_switch(&self) {
        self.kill_switch_triggered.store(true, Ordering::SeqCst);
        self.should_stop.store(true, Ordering::SeqCst);
    }

    pub fn clear_kill_switch(&self) {
        self.kill_switch_triggered.store(false, Ordering::SeqCst);
    }

    pub fn is_kill_switch_triggered(&self) -> bool {
        self.kill_switch_triggered.load(Ordering::SeqCst)
    }

    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::SeqCst)
    }

    pub fn request_pause(&self) {
        self.should_pause.store(true, Ordering::SeqCst);
    }

    pub fn should_pause(&self) -> bool {
        self.should_pause.load(Ordering::SeqCst)
    }

    pub fn resume(&self) {
        self.should_pause.store(false, Ordering::SeqCst);
    }

    pub async fn reset(&self) {
        self.metrics.reset();
        let mut state = self.state.write().await;
        *state = AgentState::default();
        self.should_stop.store(false, Ordering::SeqCst);
        self.should_pause.store(false, Ordering::SeqCst);
        self.kill_switch_triggered.store(false, Ordering::SeqCst);
        drop(state);
        self.history.clear().await;
    }

    pub async fn set_pending_action(&self, action: Option<String>) {
        let mut state = self.state.write().await;
        state.pending_action = action;
    }

    pub async fn send_confirmation(&self, response: ConfirmationResponse) -> Result<(), String> {
        let tx_guard = self.confirmation_tx.read().await;
        if let Some(tx) = tx_guard.as_ref() {
            tx.send(response)
                .await
                .map_err(|_| "Failed to send confirmation response".to_string())
        } else {
            Err("No confirmation channel available".to_string())
        }
    }

    pub async fn await_confirmation(&self) -> Option<ConfirmationResponse> {
        let mut rx_guard = self.confirmation_rx.write().await;
        if let Some(rx) = rx_guard.as_mut() {
            rx.recv().await
        } else {
            None
        }
    }

    pub async fn reset_confirmation_channel(&self) {
        let (tx, rx) = mpsc::channel(1);
        *self.confirmation_tx.write().await = Some(tx);
        *self.confirmation_rx.write().await = Some(rx);
    }

    pub async fn increment_retry(&self) -> u32 {
        let mut state = self.state.write().await;
        state.retry_count += 1;
        state.retry_count
    }

    pub async fn reset_retry_count(&self) {
        let mut state = self.state.write().await;
        state.retry_count = 0;
    }

    pub async fn increment_consecutive_errors(&self) -> u32 {
        let mut state = self.state.write().await;
        state.consecutive_errors += 1;
        state.consecutive_errors
    }

    pub async fn reset_consecutive_errors(&self) {
        let mut state = self.state.write().await;
        state.consecutive_errors = 0;
    }

    pub async fn get_consecutive_errors(&self) -> u32 {
        let state = self.state.read().await;
        state.consecutive_errors
    }

    pub async fn set_queue_info(&self, index: usize, total: usize, active: bool) {
        let mut state = self.state.write().await;
        state.queue_index = index;
        state.queue_total = total;
        state.queue_active = active;
    }

    pub async fn update_queue_progress(&self, index: usize, total: usize) {
        let mut state = self.state.write().await;
        state.queue_index = index;
        state.queue_total = total;
    }

    pub async fn set_preview_mode(&self, enabled: bool) {
        let mut state = self.state.write().await;
        state.preview_mode = enabled;
    }

    pub async fn is_preview_mode(&self) -> bool {
        let state = self.state.read().await;
        state.preview_mode
    }
}
