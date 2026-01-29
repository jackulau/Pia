mod agent;
mod capture;
mod config;
mod input;
mod llm;

use agent::{AgentLoop, AgentStateManager, AgentStatus};
use config::Config;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri::State;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = Config::load().unwrap_or_default();
    let hotkey = config.general.global_hotkey.clone();

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
            };
            app.manage(state);

            // Register global hotkey if configured
            if let Some(ref shortcut_str) = hotkey {
                match register_global_shortcut(app.handle(), shortcut_str) {
                    Ok(_) => println!("Global hotkey registered: {}", shortcut_str),
                    Err(e) => println!("Failed to register global hotkey '{}': {}", shortcut_str, e),
                }
            }

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
            get_current_hotkey,
            set_global_hotkey,
            unregister_global_hotkey,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
