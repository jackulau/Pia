mod agent;
mod capture;
mod config;
mod history;
mod input;
mod llm;

use agent::{AgentLoop, AgentStateManager, AgentStatus, ConfirmationResponse, InstructionQueue, QueueFailureMode, QueueManager};
use config::Config;
use history::{HistoryEntry, InstructionHistory};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, State,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tokio::sync::RwLock;

struct AppState {
    agent_state: AgentStateManager,
    config: Arc<RwLock<Config>>,
    history: Arc<RwLock<InstructionHistory>>,
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
    pending_action: Option<String>,
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
async fn pause_agent(state: State<'_, AppState>) -> Result<(), String> {
    state.agent_state.request_pause();
    Ok(())
}

#[tauri::command]
async fn resume_agent(state: State<'_, AppState>) -> Result<(), String> {
    state.agent_state.resume();
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
        pending_action: s.pending_action,
        tokens_per_second: s.tokens_per_second,
        total_input_tokens: s.total_input_tokens,
        total_output_tokens: s.total_output_tokens,
        queue_index: s.queue_index,
        queue_total: s.queue_total,
        queue_active: s.queue_active,
    })
}

#[tauri::command]
async fn confirm_action(state: State<'_, AppState>) -> Result<(), String> {
    state
        .agent_state
        .send_confirmation(ConfirmationResponse::Confirmed)
        .await
}

#[tauri::command]
async fn deny_action(state: State<'_, AppState>) -> Result<(), String> {
    state
        .agent_state
        .send_confirmation(ConfirmationResponse::Denied)
        .await
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
async fn save_config(config: Config, app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let show_overlay = config.general.show_coordinate_overlay;
    config.save().map_err(|e| e.to_string())?;
    *state.config.write().await = config;

    // Update overlay window visibility based on setting
    if let Some(overlay) = app_handle.get_webview_window("overlay") {
        if show_overlay {
            let _ = overlay.show();
        } else {
            let _ = overlay.hide();
        }
    }

    Ok(())
}

#[tauri::command]
async fn hide_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_session_json(
    include_screenshots: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state
        .agent_state
        .history()
        .export_json(include_screenshots)
        .await
        .ok_or_else(|| "No session history available".to_string())
}

#[tauri::command]
async fn export_session_text(state: State<'_, AppState>) -> Result<String, String> {
    state
        .agent_state
        .history()
        .export_text()
        .await
        .ok_or_else(|| "No session history available".to_string())
}

#[tauri::command]
async fn get_session_history_count(state: State<'_, AppState>) -> Result<usize, String> {
    Ok(state.agent_state.history().get_entry_count().await)
}

#[tauri::command]
async fn clear_session_history(state: State<'_, AppState>) -> Result<(), String> {
    state.agent_state.history().clear().await;
    Ok(())
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

/// Parse a shortcut string like "CmdOrCtrl+Shift+P" into a Shortcut
fn parse_shortcut(shortcut_str: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    if parts.is_empty() {
        return Err("Empty shortcut string".to_string());
    }

    let mut modifiers = Modifiers::empty();
    let key_str = parts.last().ok_or("No key specified")?;

    for part in &parts[..parts.len() - 1] {
        match part.to_lowercase().as_str() {
            "cmd" | "command" | "super" | "meta" => modifiers |= Modifiers::META,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "cmdorctrl" | "commandorcontrol" => {
                #[cfg(target_os = "macos")]
                {
                    modifiers |= Modifiers::META;
                }
                #[cfg(not(target_os = "macos"))]
                {
                    modifiers |= Modifiers::CONTROL;
                }
            }
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            _ => return Err(format!("Unknown modifier: {}", part)),
        }
    }

    let code = match key_str.to_uppercase().as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "SPACE" => Code::Space,
        "ENTER" | "RETURN" => Code::Enter,
        "TAB" => Code::Tab,
        "ESCAPE" | "ESC" => Code::Escape,
        "BACKSPACE" => Code::Backspace,
        "DELETE" => Code::Delete,
        _ => return Err(format!("Unknown key: {}", key_str)),
    };

    Ok(Shortcut::new(Some(modifiers), code))
}

/// Toggle window visibility based on current state
fn toggle_window(window: &WebviewWindow) {
    if let Ok(is_visible) = window.is_visible() {
        if is_visible {
            if let Ok(is_focused) = window.is_focused() {
                if is_focused {
                    // Window visible and focused -> hide
                    let _ = window.hide();
                } else {
                    // Window visible but not focused -> focus
                    let _ = window.set_focus();
                }
            } else {
                // Can't determine focus state, just focus
                let _ = window.set_focus();
            }
        } else {
            // Window hidden -> show and focus
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

/// Register a global shortcut for the app
fn register_global_shortcut(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    let shortcut = parse_shortcut(shortcut_str)?;
    let window = app.get_webview_window("main")
        .ok_or("Main window not found")?;

    app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            toggle_window(&window);
        }
    }).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_current_hotkey(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.config.read().await.general.global_hotkey.clone())
}

#[tauri::command]
async fn set_global_hotkey(
    shortcut: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Validate the shortcut first
    let _ = parse_shortcut(&shortcut)?;

    // Unregister all existing shortcuts
    let _ = app_handle.global_shortcut().unregister_all();

    // Register the new shortcut
    register_global_shortcut(&app_handle, &shortcut)?;

    // Update config
    let mut config = state.config.write().await;
    config.general.global_hotkey = Some(shortcut);
    config.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_instruction_history(state: State<'_, AppState>) -> Result<Vec<HistoryEntry>, String> {
    let history = state.history.read().await;
    Ok(history.get_all().to_vec())
}

#[tauri::command]
async fn add_to_history(
    instruction: String,
    success: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut history = state.history.write().await;
    history.add(instruction, success);
    history.save().map_err(|e| e.to_string())?;
    Ok(())
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
async fn unregister_global_hotkey(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    app_handle.global_shortcut().unregister_all()
        .map_err(|e| e.to_string())?;

    // Update config
    let mut config = state.config.write().await;
    config.general.global_hotkey = None;
    config.save().map_err(|e| e.to_string())?;

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

#[tauri::command]
async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let mut history = state.history.write().await;
    history.clear();
    history.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn remove_from_history(index: usize, state: State<'_, AppState>) -> Result<(), String> {
    let mut history = state.history.write().await;
    if history.remove(index) {
        history.save().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Invalid history index".to_string())
    }
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = Config::load().unwrap_or_default();
    let show_overlay_at_startup = config.general.show_coordinate_overlay;
    let hotkey = config.general.global_hotkey.clone();
    let history = InstructionHistory::load().unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
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
                history: Arc::new(RwLock::new(history)),
                queue: QueueManager::new(),
            };
            app.manage(state);

            // Create tray menu
            let show_hide = MenuItem::with_id(app, "show_hide", "Show/Hide Pia", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_hide, &quit])?;

            // Create tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Pia - Computer Use Agent")
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            // Toggle window visibility on left click
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show_hide" => {
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Register global hotkey if configured
            if let Some(ref shortcut_str) = hotkey {
                match register_global_shortcut(app.handle(), shortcut_str) {
                    Ok(_) => println!("Global hotkey registered: {}", shortcut_str),
                    Err(e) => println!("Failed to register global hotkey '{}': {}", shortcut_str, e),
                }
            }

            // Show main window on startup
            if let Some(window) = app.get_webview_window("main") {
                println!("Window found, showing...");
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                println!("ERROR: No window found!");
            }

            // Show overlay window if enabled in config
            if show_overlay_at_startup {
                if let Some(overlay) = app.get_webview_window("overlay") {
                    let _ = overlay.show();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_agent,
            stop_agent,
            pause_agent,
            resume_agent,
            get_agent_state,
            get_config,
            save_config,
            hide_window,
            show_window,
            confirm_action,
            deny_action,
            show_cursor_indicator,
            hide_cursor_indicator,
            get_current_hotkey,
            set_global_hotkey,
            unregister_global_hotkey,
            export_session_json,
            export_session_text,
            get_session_history_count,
            clear_session_history,
            get_instruction_history,
            add_to_history,
            clear_history,
            remove_from_history,
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
