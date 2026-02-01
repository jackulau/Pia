use crate::input::{
    is_dangerous_key_combination, parse_key, parse_modifier, KeyboardController, Modifier,
    MouseButton, MouseController, ScrollDirection,
};
use crate::llm::provider::{LlmResponse, ToolUse};
use super::retry::{RetryContext, RetryError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::thread;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActionError {
    #[error("Failed to parse action: {0}")]
    ParseError(String),
    #[error("Mouse action failed: {0}")]
    MouseError(#[from] crate::input::MouseError),
    #[error("Keyboard action failed: {0}")]
    KeyboardError(#[from] crate::input::KeyboardError),
    #[error("Action requires confirmation: {0}")]
    RequiresConfirmation(String),
    #[error("Unknown action type: {0}")]
    UnknownAction(String),
    #[error("Retry error: {0}")]
    RetryError(#[from] RetryError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    Click {
        x: i32,
        y: i32,
        #[serde(default = "default_button")]
        button: String,
    },
    DoubleClick {
        x: i32,
        y: i32,
    },
    Move {
        x: i32,
        y: i32,
    },
    Type {
        text: String,
    },
    Key {
        key: String,
        #[serde(default)]
        modifiers: Vec<String>,
    },
    Scroll {
        x: i32,
        y: i32,
        direction: String,
        #[serde(default = "default_scroll_amount")]
        amount: i32,
    },
    Drag {
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        #[serde(default = "default_button")]
        button: String,
        #[serde(default = "default_drag_duration")]
        duration_ms: u32,
    },
    TripleClick {
        x: i32,
        y: i32,
    },
    RightClick {
        x: i32,
        y: i32,
    },
    Wait {
        #[serde(default = "default_wait_duration")]
        duration_ms: u64,
    },
    Complete {
        message: String,
    },
    Error {
        message: String,
    },
    Batch {
        actions: Vec<Action>,
    },
}

fn default_button() -> String {
    "left".to_string()
}

fn default_scroll_amount() -> i32 {
    3
}

const MAX_BATCH_SIZE: usize = 10;
const BATCH_INTER_ACTION_DELAY_MS: u64 = 100;

fn default_drag_duration() -> u32 {
    500
}

fn default_wait_duration() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub completed: bool,
    pub message: Option<String>,
    #[serde(default)]
    pub retry_count: u32,
}

/// Parse an action from an LLM response (either tool_use or text)
pub fn parse_llm_response(response: &LlmResponse) -> Result<Action, ActionError> {
    match response {
        LlmResponse::ToolUse(tool_use) => from_tool_use(tool_use),
        LlmResponse::Text(text) => parse_action(text),
    }
}

/// Parse an action from a native tool_use response
pub fn from_tool_use(tool_use: &ToolUse) -> Result<Action, ActionError> {
    let input = &tool_use.input;

    match tool_use.name.as_str() {
        "click" => {
            let x = get_i32(input, "x")?;
            let y = get_i32(input, "y")?;
            let button = get_string_or_default(input, "button", "left");
            Ok(Action::Click { x, y, button })
        }
        "double_click" => {
            let x = get_i32(input, "x")?;
            let y = get_i32(input, "y")?;
            Ok(Action::DoubleClick { x, y })
        }
        "move" => {
            let x = get_i32(input, "x")?;
            let y = get_i32(input, "y")?;
            Ok(Action::Move { x, y })
        }
        "type" => {
            let text = get_string(input, "text")?;
            Ok(Action::Type { text })
        }
        "key" => {
            let key = get_string(input, "key")?;
            let modifiers = get_string_array_or_default(input, "modifiers");
            Ok(Action::Key { key, modifiers })
        }
        "scroll" => {
            let x = get_i32(input, "x")?;
            let y = get_i32(input, "y")?;
            let direction = get_string(input, "direction")?;
            let amount = get_i32_or_default(input, "amount", 3);
            Ok(Action::Scroll {
                x,
                y,
                direction,
                amount,
            })
        }
        "complete" => {
            let message = get_string(input, "message")?;
            Ok(Action::Complete { message })
        }
        "error" => {
            let message = get_string(input, "message")?;
            Ok(Action::Error { message })
        }
        _ => Err(ActionError::UnknownAction(tool_use.name.clone())),
    }
}

// Helper functions for extracting values from JSON
fn get_i32(value: &Value, key: &str) -> Result<i32, ActionError> {
    value
        .get(key)
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| ActionError::ParseError(format!("Missing or invalid field: {}", key)))
}

fn get_i32_or_default(value: &Value, key: &str, default: i32) -> i32 {
    value
        .get(key)
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(default)
}

fn get_string(value: &Value, key: &str) -> Result<String, ActionError> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ActionError::ParseError(format!("Missing or invalid field: {}", key)))
}

fn get_string_or_default(value: &Value, key: &str, default: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default.to_string())
}

fn get_string_array_or_default(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Parse an action from raw JSON text (fallback for non-tool providers)
pub fn parse_action(response: &str) -> Result<Action, ActionError> {
    // Try to find JSON in the response
    let json_str = extract_json(response)?;

    serde_json::from_str(&json_str)
        .map_err(|e| ActionError::ParseError(format!("Invalid JSON: {} in '{}'", e, json_str)))
}

fn extract_json(text: &str) -> Result<String, ActionError> {
    // Find the first { and matching }
    let start = text
        .find('{')
        .ok_or_else(|| ActionError::ParseError("No JSON object found".to_string()))?;

    let mut depth = 0;
    let mut end = start;

    for (i, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Err(ActionError::ParseError("Unbalanced braces".to_string()));
    }

    Ok(text[start..end].to_string())
}

pub async fn execute_action(
    action: &Action,
    confirm_dangerous: bool,
) -> Result<ActionResult, ActionError> {
    match action {
        Action::Click { x, y, button } => {
            let btn = match button.to_lowercase().as_str() {
                "left" => MouseButton::Left,
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };

            let x = *x;
            let y = *y;
            let button_str = button.clone();

            tokio::task::spawn_blocking(move || {
                let mut mouse = MouseController::new()?;
                mouse.click_at(x, y, btn)
            })
            .await
            .map_err(|e| ActionError::MouseError(crate::input::MouseError::ActionError(e.to_string())))??;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Clicked {} at ({}, {})", button_str, x, y)),
                retry_count: 0,
            })
        }

        Action::DoubleClick { x, y } => {
            let mut mouse = MouseController::new()?;
            mouse.move_to(*x, *y)?;
            mouse.double_click(MouseButton::Left)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Double-clicked at ({}, {})", x, y)),
                retry_count: 0,
            })
        }

        Action::Move { x, y } => {
            let mut mouse = MouseController::new()?;
            mouse.move_to(*x, *y)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Moved mouse to ({}, {})", x, y)),
                retry_count: 0,
            })
        }

        Action::Type { text } => {
            let mut keyboard = KeyboardController::new()?;
            keyboard.type_text(text)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Typed: {}", truncate_string(text, 50))),
                retry_count: 0,
            })
        }

        Action::Key { key, modifiers } => {
            let mods: Vec<Modifier> = modifiers
                .iter()
                .filter_map(|m| parse_modifier(m))
                .collect();

            // Check for dangerous combinations
            if confirm_dangerous && is_dangerous_key_combination(key, &mods) {
                return Err(ActionError::RequiresConfirmation(format!(
                    "Dangerous key combination: {} + {:?}",
                    key, modifiers
                )));
            }

            let mut keyboard = KeyboardController::new()?;

            if mods.is_empty() {
                let k = parse_key(key)?;
                keyboard.key_press(k)?;
            } else {
                keyboard.key_with_modifiers(key, &mods)?;
            }

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Pressed key: {} with modifiers: {:?}", key, modifiers)),
                retry_count: 0,
            })
        }

        Action::Scroll {
            x,
            y,
            direction,
            amount,
        } => {
            let dir = match direction.to_lowercase().as_str() {
                "up" => ScrollDirection::Up,
                "down" => ScrollDirection::Down,
                "left" => ScrollDirection::Left,
                "right" => ScrollDirection::Right,
                _ => ScrollDirection::Down,
            };

            let x = *x;
            let y = *y;
            let amount = *amount;
            let direction_str = direction.clone();

            tokio::task::spawn_blocking(move || {
                let mut mouse = MouseController::new()?;
                mouse.move_to(x, y)?;
                std::thread::sleep(std::time::Duration::from_millis(50));
                mouse.scroll(dir, amount)
            })
            .await
            .map_err(|e| ActionError::MouseError(crate::input::MouseError::ActionError(e.to_string())))??;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Scrolled {} {} times at ({}, {})", direction_str, amount, x, y)),
                retry_count: 0,
            })
        }

        Action::Drag {
            start_x,
            start_y,
            end_x,
            end_y,
            button,
            duration_ms,
        } => {
            let btn = match button.to_lowercase().as_str() {
                "left" => MouseButton::Left,
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };

            // Cap duration at 5 seconds
            let duration = (*duration_ms).min(5000);

            log::info!(
                "Drag {} from ({}, {}) to ({}, {}) over {}ms",
                button,
                start_x,
                start_y,
                end_x,
                end_y,
                duration
            );

            let mut mouse = MouseController::new()?;
            mouse.drag(*start_x, *start_y, *end_x, *end_y, btn, duration)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!(
                    "Dragged from ({}, {}) to ({}, {})",
                    start_x, start_y, end_x, end_y
                )),
                retry_count: 0,
            })
        }

        Action::TripleClick { x, y } => {
            let mut mouse = MouseController::new()?;
            mouse.move_to(*x, *y)?;
            mouse.triple_click(MouseButton::Left)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Triple-clicked at ({}, {})", x, y)),
                retry_count: 0,
            })
        }

        Action::RightClick { x, y } => {
            let mut mouse = MouseController::new()?;
            mouse.click_at(*x, *y, MouseButton::Right)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Right-clicked at ({}, {})", x, y)),
                retry_count: 0,
            })
        }

        Action::Wait { duration_ms } => {
            std::thread::sleep(std::time::Duration::from_millis(*duration_ms));

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Waited {} ms", duration_ms)),
                retry_count: 0,
            })
        }

        Action::Complete { message } => Ok(ActionResult {
            success: true,
            completed: true,
            message: Some(message.clone()),
            retry_count: 0,
        }),

        Action::Error { message } => Ok(ActionResult {
            success: false,
            completed: true,
            message: Some(message.clone()),
            retry_count: 0,
        }),

        Action::Batch { actions } => {
            if actions.is_empty() {
                return Ok(ActionResult {
                    success: true,
                    completed: false,
                    message: Some("Empty batch, nothing to execute".into()),
                });
            }

            if actions.len() > MAX_BATCH_SIZE {
                return Ok(ActionResult {
                    success: false,
                    completed: false,
                    message: Some(format!(
                        "Batch size {} exceeds maximum of {}",
                        actions.len(),
                        MAX_BATCH_SIZE
                    )),
                });
            }

            for (i, sub_action) in actions.iter().enumerate() {
                // Prevent nested batches
                if matches!(sub_action, Action::Batch { .. }) {
                    return Ok(ActionResult {
                        success: false,
                        completed: false,
                        message: Some("Nested batches are not allowed".into()),
                    });
                }

                let result = Box::pin(execute_action(sub_action, confirm_dangerous)).await?;

                if !result.success {
                    return Ok(ActionResult {
                        success: false,
                        completed: false,
                        message: Some(format!(
                            "Batch failed at action {}/{}: {}",
                            i + 1,
                            actions.len(),
                            result.message.unwrap_or_default()
                        )),
                    });
                }

                if result.completed {
                    return Ok(ActionResult {
                        success: true,
                        completed: true,
                        message: Some(format!(
                            "Batch completed early at action {}/{}: {}",
                            i + 1,
                            actions.len(),
                            result.message.unwrap_or_default()
                        )),
                    });
                }

                // Small delay between batched actions (except after the last one)
                if i < actions.len() - 1 {
                    std::thread::sleep(std::time::Duration::from_millis(BATCH_INTER_ACTION_DELAY_MS));
                }
            }

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Batch completed: {} actions executed", actions.len())),
            })
        }
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

impl Action {
    /// Returns true if this action should produce visible screen changes
    /// and should be verified after execution.
    pub fn should_verify_effect(&self) -> bool {
        matches!(
            self,
            Action::Click { .. }
                | Action::DoubleClick { .. }
                | Action::Type { .. }
                | Action::Key { .. }
                | Action::Scroll { .. }
        )
    }
}

/// Execute an action with retry logic.
/// Automatically retries failed actions or actions that don't produce
/// visible screen changes.
pub fn execute_action_with_retry(
    action: &Action,
    confirm_dangerous: bool,
    retry_ctx: &mut RetryContext,
) -> Result<ActionResult, ActionError> {
    // Reset retry context for this action
    retry_ctx.reset();

    loop {
        // Capture before state for actions that should change the screen
        if action.should_verify_effect() {
            retry_ctx.capture_before()?;
        }

        // Execute the action
        let mut result = execute_action(action, confirm_dangerous)?;

        // If action failed and we can retry
        if !result.success {
            if retry_ctx.should_retry() {
                retry_ctx.increment();
                log::warn!(
                    "Action failed, retrying ({}/{}): {:?}",
                    retry_ctx.attempt,
                    retry_ctx.max_retries,
                    action
                );
                thread::sleep(retry_ctx.retry_delay);
                continue;
            }
            result.retry_count = retry_ctx.attempt;
            return Ok(result);
        }

        // For actions that should have visible effect, verify screen changed
        if action.should_verify_effect() && retry_ctx.enabled {
            // Wait a bit for UI to update
            thread::sleep(Duration::from_millis(200));

            if !retry_ctx.screen_changed()? {
                if retry_ctx.should_retry() {
                    retry_ctx.increment();
                    log::warn!(
                        "Action had no visible effect, retrying ({}/{}): {:?}",
                        retry_ctx.attempt,
                        retry_ctx.max_retries,
                        action
                    );
                    thread::sleep(retry_ctx.retry_delay);
                    continue;
                }
                log::warn!("Action completed but no screen change detected after {} retries", retry_ctx.attempt);
            }
        }

        result.retry_count = retry_ctx.attempt;
        return Ok(result);
    }
}
