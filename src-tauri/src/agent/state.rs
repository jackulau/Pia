use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
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
    pub kill_switch_triggered: bool,
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
            kill_switch_triggered: false,
        }
    }
}

pub struct AgentStateManager {
    state: Arc<RwLock<AgentState>>,
    should_stop: Arc<AtomicBool>,
    kill_switch_triggered: Arc<AtomicBool>,
}

impl Clone for AgentStateManager {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            should_stop: Arc::clone(&self.should_stop),
            kill_switch_triggered: Arc::clone(&self.kill_switch_triggered),
        }
    }
}

impl AgentStateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AgentState::default())),
            should_stop: Arc::new(AtomicBool::new(false)),
            kill_switch_triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn get_state(&self) -> AgentState {
        let mut state = self.state.read().await.clone();
        state.kill_switch_triggered = self.kill_switch_triggered.load(Ordering::SeqCst);
        state
    }

    pub async fn set_status(&self, status: AgentStatus) {
        let mut state = self.state.write().await;
        state.status = status;
    }

    pub async fn start(&self, instruction: String, max_iterations: u32) {
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
        self.should_stop.store(false, Ordering::SeqCst);
        self.kill_switch_triggered.store(false, Ordering::SeqCst);
    }

    pub async fn increment_iteration(&self) -> u32 {
        let mut state = self.state.write().await;
        state.iteration += 1;
        state.iteration
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

    pub async fn update_metrics(&self, tokens_per_sec: f64, input_tokens: u64, output_tokens: u64) {
        let mut state = self.state.write().await;
        state.tokens_per_second = tokens_per_sec;
        state.total_input_tokens += input_tokens;
        state.total_output_tokens += output_tokens;
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

    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = AgentState::default();
        self.should_stop.store(false, Ordering::SeqCst);
        self.kill_switch_triggered.store(false, Ordering::SeqCst);
    }
}
