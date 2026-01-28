use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
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
    Paused,
    AwaitingConfirmation,
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
    pub pending_action: Option<String>,
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
            pending_action: None,
            tokens_per_second: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
        }
    }
}

pub struct AgentStateManager {
    state: Arc<RwLock<AgentState>>,
    should_stop: Arc<AtomicBool>,
    confirmation_tx: Arc<RwLock<Option<mpsc::Sender<ConfirmationResponse>>>>,
    confirmation_rx: Arc<RwLock<Option<mpsc::Receiver<ConfirmationResponse>>>>,
}

impl Clone for AgentStateManager {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            should_stop: Arc::clone(&self.should_stop),
            confirmation_tx: Arc::clone(&self.confirmation_tx),
            confirmation_rx: Arc::clone(&self.confirmation_rx),
        }
    }
}

impl AgentStateManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);
        Self {
            state: Arc::new(RwLock::new(AgentState::default())),
            should_stop: Arc::new(AtomicBool::new(false)),
            confirmation_tx: Arc::new(RwLock::new(Some(tx))),
            confirmation_rx: Arc::new(RwLock::new(Some(rx))),
        }
    }

    pub async fn get_state(&self) -> AgentState {
        self.state.read().await.clone()
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

    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::SeqCst)
    }

    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = AgentState::default();
        self.should_stop.store(false, Ordering::SeqCst);
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
}
