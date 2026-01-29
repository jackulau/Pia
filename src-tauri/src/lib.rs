mod agent;
mod capture;
mod config;
mod input;
mod llm;

use agent::{ActionHistory, AgentLoop, AgentStateManager, AgentStatus};
use agent::action::execute_action;
use config::Config;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri::State;
use tokio::sync::RwLock;

struct AppState {
    agent_state: AgentStateManager,
    config: Arc<RwLock<Config>>,
    action_history: Arc<RwLock<ActionHistory>>,
}

#[derive(Clone, Serialize)]
struct AgentStatePayload {
    status: String,
    instruction: Option<String>,
    iteration: u32,
    max_iterations: u32,
    last_action: Option<String>,
    last_error: Option<String>,
    tokens_per_second: f64,
    total_input_tokens: u64,
    total_output_tokens: u64,
    can_undo: bool,
    last_undoable_action: Option<String>,
}

#[tauri::command]
async fn start_agent(
    instruction: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let agent_state = state.agent_state.clone();
    let config = state.config.read().await.clone();
    let action_history = state.action_history.clone();

    let current_state = agent_state.get_state().await;
    if current_state.status == AgentStatus::Running {
        return Err("Agent is already running".to_string());
    }

    let app = app_handle.clone();
    tokio::spawn(async move {
        let loop_runner = AgentLoop::new(agent_state, config, app, action_history);
        if let Err(e) = loop_runner.run(instruction).await {
            log::error!("Agent loop error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_agent(state: State<'_, AppState>) -> Result<(), String> {
    state.agent_state.request_stop();
    Ok(())
}

#[tauri::command]
async fn get_agent_state(state: State<'_, AppState>) -> Result<AgentStatePayload, String> {
    let s = state.agent_state.get_state().await;
    Ok(AgentStatePayload {
        status: format!("{:?}", s.status),
        instruction: s.instruction,
        iteration: s.iteration,
        max_iterations: s.max_iterations,
        last_action: s.last_action,
        last_error: s.last_error,
        tokens_per_second: s.tokens_per_second,
        total_input_tokens: s.total_input_tokens,
        total_output_tokens: s.total_output_tokens,
        can_undo: s.can_undo,
        last_undoable_action: s.last_undoable_action,
    })
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
async fn save_config(config: Config, state: State<'_, AppState>) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    *state.config.write().await = config;
    Ok(())
}

#[tauri::command]
async fn undo_last_action(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Check if agent is running - can't undo during execution
    let current_state = state.agent_state.get_state().await;
    if current_state.status == AgentStatus::Running {
        return Err("Cannot undo while agent is running".to_string());
    }

    // Pop the last action from history
    let record = {
        let mut history = state.action_history.write().await;
        history.pop_last()
    };

    let record = match record {
        Some(r) => r,
        None => return Err("No actions to undo".to_string()),
    };

    // Check if the action is reversible
    let reverse_action = match record.reverse_action {
        Some(action) => action,
        None => return Err(format!("Action '{}' cannot be undone", record.description)),
    };

    // Execute the reverse action
    match execute_action(&reverse_action, false) {
        Ok(_result) => {
            // Update undo state after successful undo
            let history = state.action_history.read().await;
            let can_undo = history.can_undo();
            let last_undoable = history.get_last_undoable_description();
            drop(history);

            state
                .agent_state
                .update_undo_state(can_undo, last_undoable)
                .await;

            // Emit state update
            let s = state.agent_state.get_state().await;
            let payload = AgentStatePayload {
                status: format!("{:?}", s.status),
                instruction: s.instruction,
                iteration: s.iteration,
                max_iterations: s.max_iterations,
                last_action: Some(format!("Undone: {}", record.description)),
                last_error: s.last_error,
                tokens_per_second: s.tokens_per_second,
                total_input_tokens: s.total_input_tokens,
                total_output_tokens: s.total_output_tokens,
                can_undo: s.can_undo,
                last_undoable_action: s.last_undoable_action,
            };
            let _ = app_handle.emit("agent-state", payload);

            Ok(format!("Undone: {}", record.description))
        }
        Err(e) => Err(format!("Failed to undo: {}", e)),
    }
}

#[tauri::command]
async fn hide_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
async fn show_window(window: WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = Config::load().unwrap_or_default();

    tauri::Builder::default()
        // Temporarily disable global shortcuts to test
        // .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            println!("Pia starting up...");
            
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let state = AppState {
                agent_state: AgentStateManager::new(),
                config: Arc::new(RwLock::new(config)),
                action_history: Arc::new(RwLock::new(ActionHistory::default())),
            };
            app.manage(state);

            // Show window on startup
            if let Some(window) = app.get_webview_window("main") {
                println!("Window found, showing...");
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                println!("ERROR: No window found!");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_agent,
            stop_agent,
            get_agent_state,
            get_config,
            save_config,
            hide_window,
            show_window,
            undo_last_action,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
