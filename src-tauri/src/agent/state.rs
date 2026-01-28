use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub status: AgentStatus,
    pub instruction: Option<String>,
    pub iteration: u32,
    pub max_iterations: u32,
    pub last_action: Option<String>,
    pub last_error: Option<String>,
    pub tokens_per_second: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            status: AgentStatus::Idle,
            instruction: None,
            iteration: 0,
            max_iterations: 50,
            last_action: None,
            last_error: None,
            tokens_per_second: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
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
}

impl Clone for AgentStateManager {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            metrics: Arc::clone(&self.metrics),
            should_stop: Arc::clone(&self.should_stop),
        }
    }
}

impl AgentStateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AgentState::default())),
            metrics: Arc::new(AtomicMetrics::new()),
            should_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the full state, merging atomic metrics with RwLock-protected fields.
    /// Use specific getters when only individual values are needed.
    pub async fn get_state(&self) -> AgentState {
        let state = self.state.read().await;
        AgentState {
            status: state.status,
            instruction: state.instruction.clone(),
            iteration: self.metrics.iteration.load(Ordering::Acquire),
            max_iterations: self.metrics.max_iterations.load(Ordering::Acquire),
            last_action: state.last_action.clone(),
            last_error: state.last_error.clone(),
            tokens_per_second: self.metrics.get_tokens_per_second(),
            total_input_tokens: self.metrics.total_input_tokens.load(Ordering::Acquire),
            total_output_tokens: self.metrics.total_output_tokens.load(Ordering::Acquire),
        }
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
        // Reset atomic metrics first (no lock needed)
        self.metrics.iteration.store(0, Ordering::Release);
        self.metrics.max_iterations.store(max_iterations, Ordering::Release);
        self.metrics.total_input_tokens.store(0, Ordering::Release);
        self.metrics.total_output_tokens.store(0, Ordering::Release);
        self.metrics.set_tokens_per_second(0.0);
        self.should_stop.store(false, Ordering::SeqCst);

        // Now update the RwLock-protected state
        let mut state = self.state.write().await;
        state.status = AgentStatus::Running;
        state.instruction = Some(instruction);
        state.iteration = 0;
        state.max_iterations = max_iterations;
        state.last_action = None;
        state.last_error = None;
        state.tokens_per_second = 0.0;
        state.total_input_tokens = 0;
        state.total_output_tokens = 0;
    }

    /// Atomically increment iteration counter without acquiring RwLock
    pub fn increment_iteration(&self) -> u32 {
        self.metrics.iteration.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub async fn set_last_action(&self, action: String) {
        let mut state = self.state.write().await;
        state.last_action = Some(action);
    }

    pub async fn set_error(&self, error: String) {
        let mut state = self.state.write().await;
        state.status = AgentStatus::Error;
        state.last_error = Some(error);
    }

    /// Update metrics atomically without acquiring RwLock
    pub fn update_metrics(&self, tokens_per_sec: f64, input_tokens: u64, output_tokens: u64) {
        self.metrics.set_tokens_per_second(tokens_per_sec);
        self.metrics.total_input_tokens.fetch_add(input_tokens, Ordering::AcqRel);
        self.metrics.total_output_tokens.fetch_add(output_tokens, Ordering::AcqRel);
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

    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::SeqCst)
    }

    pub async fn reset(&self) {
        self.metrics.reset();
        let mut state = self.state.write().await;
        *state = AgentState::default();
        self.should_stop.store(false, Ordering::SeqCst);
    }
}
