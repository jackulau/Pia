mod agent;
mod capture;
mod config;
mod input;
mod llm;

use agent::{AgentLoop, AgentStateManager, AgentStatus, InstructionQueue, QueueFailureMode, QueueManager};
use config::Config;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri::State;
use tokio::sync::RwLock;

struct AppState {
    agent_state: AgentStateManager,
    config: Arc<RwLock<Config>>,
    queue: QueueManager,
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
    queue_index: usize,
    queue_total: usize,
    queue_active: bool,
}

#[tauri::command]
async fn start_agent(
    instruction: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let agent_state = state.agent_state.clone();
    let config = state.config.read().await.clone();

    let current_state = agent_state.get_state().await;
    if current_state.status == AgentStatus::Running {
        return Err("Agent is already running".to_string());
    }

    let app = app_handle.clone();
    tokio::spawn(async move {
        let loop_runner = AgentLoop::new(agent_state, config, app);
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
        queue_index: s.queue_index,
        queue_total: s.queue_total,
        queue_active: s.queue_active,
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
async fn hide_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
async fn show_window(window: WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

// Queue commands

#[tauri::command]
async fn add_to_queue(
    instruction: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let id = state.queue.add(instruction).await;
    Ok(id)
}

#[tauri::command]
async fn add_multiple_to_queue(
    instructions: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let ids = state.queue.add_multiple(instructions).await;
    Ok(ids)
}

#[tauri::command]
async fn remove_from_queue(
    id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    Ok(state.queue.remove(&id).await)
}

#[tauri::command]
async fn clear_queue(state: State<'_, AppState>) -> Result<(), String> {
    state.queue.clear().await;
    Ok(())
}

#[tauri::command]
async fn reorder_queue(
    order: Vec<String>,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    Ok(state.queue.reorder(order).await)
}

#[tauri::command]
async fn get_queue(state: State<'_, AppState>) -> Result<InstructionQueue, String> {
    Ok(state.queue.get_state().await)
}

#[tauri::command]
async fn start_queue(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let agent_state = state.agent_state.clone();
    let config = state.config.read().await.clone();
    let queue = state.queue.clone();

    let current_state = agent_state.get_state().await;
    if current_state.status == AgentStatus::Running {
        return Err("Agent is already running".to_string());
    }

    if !queue.has_pending().await {
        return Err("Queue is empty".to_string());
    }

    let app = app_handle.clone();
    tokio::spawn(async move {
        let loop_runner = AgentLoop::new(agent_state, config, app).with_queue(queue);
        if let Err(e) = loop_runner.run_queue().await {
            log::error!("Queue processing error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
async fn set_queue_failure_mode(
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let failure_mode = QueueFailureMode::from(mode.as_str());
    state.queue.set_failure_mode(failure_mode).await;

    // Also update config
    let mut config = state.config.write().await;
    config.general.queue_failure_mode = mode;
    config.save().map_err(|e| e.to_string())?;

    Ok(())
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
                queue: QueueManager::new(),
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
            add_to_queue,
            add_multiple_to_queue,
            remove_from_queue,
            clear_queue,
            reorder_queue,
            get_queue,
            start_queue,
            set_queue_failure_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
