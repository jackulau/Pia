mod agent;
mod capture;
mod config;
mod input;
mod llm;

use agent::{AgentLoop, AgentStateManager, AgentStatus};
use config::Config;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
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

#[derive(Clone, Serialize, Deserialize)]
struct CursorIndicatorPayload {
    x: i32,
    y: i32,
    action_type: String,
}

#[tauri::command]
async fn show_cursor_indicator(
    x: i32,
    y: i32,
    action_type: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    if let Some(overlay) = app_handle.get_webview_window("cursor-overlay") {
        // Get the monitor that contains the target coordinates
        let monitors = overlay.available_monitors().map_err(|e| e.to_string())?;

        // Find the monitor containing the point, or use primary
        let target_monitor = monitors
            .iter()
            .find(|m| {
                let pos = m.position();
                let size = m.size();
                x >= pos.x && x < pos.x + size.width as i32 &&
                y >= pos.y && y < pos.y + size.height as i32
            })
            .or_else(|| monitors.first());

        if let Some(monitor) = target_monitor {
            let monitor_pos = monitor.position();
            let monitor_size = monitor.size();

            // Position the overlay to cover the monitor
            overlay.set_position(PhysicalPosition::new(monitor_pos.x, monitor_pos.y))
                .map_err(|e| e.to_string())?;
            overlay.set_size(PhysicalSize::new(monitor_size.width, monitor_size.height))
                .map_err(|e| e.to_string())?;

            // Calculate position relative to the overlay window
            let relative_x = x - monitor_pos.x;
            let relative_y = y - monitor_pos.y;

            // Show the overlay and emit the cursor position
            overlay.show().map_err(|e| e.to_string())?;

            let payload = CursorIndicatorPayload {
                x: relative_x,
                y: relative_y,
                action_type,
            };
            overlay.emit("show-cursor-indicator", payload).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn hide_cursor_indicator(app_handle: AppHandle) -> Result<(), String> {
    if let Some(overlay) = app_handle.get_webview_window("cursor-overlay") {
        overlay.emit("hide-cursor-indicator", ()).map_err(|e| e.to_string())?;
        // Hide after a short delay for animation
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        overlay.hide().map_err(|e| e.to_string())?;
    }
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
            show_cursor_indicator,
            hide_cursor_indicator,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
