mod agent;
mod capture;
mod config;
mod input;
mod llm;

use agent::{AgentLoop, AgentStateManager, AgentStatus};
use config::Config;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use tauri::State;
use tokio::sync::RwLock;

struct AppState {
    agent_state: AgentStateManager,
    config: Arc<RwLock<Config>>,
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
    preview_mode: bool,
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
        preview_mode: s.preview_mode,
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
async fn set_preview_mode(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    // Update the agent state
    state.agent_state.set_preview_mode(enabled).await;

    // Also update the config and save it
    let mut config = state.config.write().await;
    config.general.preview_mode = enabled;
    config.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_preview_mode(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.agent_state.is_preview_mode().await)
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

            let agent_state = AgentStateManager::new();
            // Initialize preview_mode from config
            {
                let preview_mode = config.general.preview_mode;
                let agent_state_clone = agent_state.clone();
                tokio::spawn(async move {
                    agent_state_clone.set_preview_mode(preview_mode).await;
                });
            }
            let state = AppState {
                agent_state,
                config: Arc::new(RwLock::new(config)),
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
            set_preview_mode,
            get_preview_mode,
            hide_window,
            show_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
