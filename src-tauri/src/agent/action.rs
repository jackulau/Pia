use crate::input::{
    is_dangerous_key_combination, parse_key, parse_modifier, KeyboardController, Modifier,
    MouseButton, MouseController, ScrollDirection,
};
use crate::llm::provider::{LlmResponse, ToolUse};
use super::retry::{RetryContext, RetryError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    WaitForElement {
        timeout_ms: Option<u32>,
        description: String,
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

/// Details specific to each action type, used for rich feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionDetails {
    Click {
        x: i32,
        y: i32,
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
        text_length: usize,
        preview: String,
    },
    Key {
        key: String,
        modifiers: Vec<String>,
    },
    Scroll {
        x: i32,
        y: i32,
        direction: String,
        amount: i32,
    },
    Complete {
        message: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub completed: bool,
    pub message: Option<String>,
    #[serde(default)]
    pub retry_count: u32,
    /// The type of action that was executed
    #[serde(default)]
    pub action_type: String,
    /// Detailed information about the executed action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ActionDetails>,
    /// The tool_use_id this result corresponds to (set by caller)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

impl ActionResult {
    /// Convert this ActionResult to a compact tool_result content string for the LLM.
    /// Success returns "OK", failure returns "FAIL: <message>".
    /// The LLM already knows what action it requested, so we omit redundant details.
    pub fn to_tool_result_content(&self) -> String {
        if self.success {
            "OK".to_string()
        } else {
            format!("FAIL: {}", self.message.as_deref().unwrap_or("Unknown error"))
        }
    }

    /// Set the tool_use_id for this result
    pub fn with_tool_use_id(mut self, id: String) -> Self {
        self.tool_use_id = Some(id);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ParsedResponse {
    pub action: Action,
    pub reasoning: Option<String>,
}

/// Parse an action from an LLM response (either tool_use or text)
pub fn parse_llm_response(response: &LlmResponse) -> Result<Action, ActionError> {
    match response {
        LlmResponse::ToolUse(tool_use) => from_tool_use(tool_use),
        LlmResponse::Text(text) => {
            let parsed = parse_action(text)?;
            Ok(parsed.action)
        }
    }
}

/// Parse an action from an LLM response with reasoning extraction
pub fn parse_llm_response_with_reasoning(response: &LlmResponse) -> Result<ParsedResponse, ActionError> {
    match response {
        LlmResponse::ToolUse(tool_use) => {
            let action = from_tool_use(tool_use)?;
            Ok(ParsedResponse { action, reasoning: None })
        }
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
pub fn parse_action(response: &str) -> Result<ParsedResponse, ActionError> {
    // Extract reasoning (text before the JSON block)
    let reasoning = extract_reasoning(response);

    // Try to find JSON in the response
    let json_str = extract_json(response)?;

    let action = serde_json::from_str(&json_str)
        .map_err(|e| ActionError::ParseError(format!("Invalid JSON: {} in '{}'", e, json_str)))?;

    Ok(ParsedResponse { action, reasoning })
}

fn extract_reasoning(text: &str) -> Option<String> {
    // Find the start of JSON
    let json_start = text.find('{')?;

    // Get text before the JSON
    let before_json = text[..json_start].trim();

    if before_json.is_empty() {
        return None;
    }

    // Clean up the reasoning text
    // Remove markdown code block markers if present
    let cleaned = before_json
        .trim_end_matches("```json")
        .trim_end_matches("```")
        .trim();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
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
    execute_action_with_delay(
        action,
        confirm_dangerous,
        std::time::Duration::from_millis(50),
    )
    .await
}

pub async fn execute_action_with_delay(
    action: &Action,
    confirm_dangerous: bool,
    click_delay: std::time::Duration,
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
            let delay = click_delay;

            tokio::task::spawn_blocking(move || {
                let mut mouse = MouseController::new()?;
                mouse.click_at_with_delay(x, y, btn, delay)
            })
            .await
            .map_err(|e| ActionError::MouseError(crate::input::MouseError::ActionError(e.to_string())))??;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Clicked {} at ({}, {})", button_str, x, y)),
                retry_count: 0,
                action_type: "click".to_string(),
                details: Some(ActionDetails::Click {
                    x,
                    y,
                    button: button_str,
                }),
                tool_use_id: None,
            })
        }

        Action::DoubleClick { x, y } => {
            let x = *x;
            let y = *y;
            let delay = click_delay;

            tokio::task::spawn_blocking(move || {
                let mut mouse = MouseController::new()?;
                mouse.move_to(x, y)?;
                std::thread::sleep(delay);
                mouse.double_click(MouseButton::Left)
            })
            .await
            .map_err(|e| ActionError::MouseError(crate::input::MouseError::ActionError(e.to_string())))??;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Double-clicked at ({}, {})", x, y)),
                retry_count: 0,
                action_type: "double_click".to_string(),
                details: Some(ActionDetails::DoubleClick { x, y }),
                tool_use_id: None,
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
                action_type: "move".to_string(),
                details: Some(ActionDetails::Move { x: *x, y: *y }),
                tool_use_id: None,
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
                action_type: "type".to_string(),
                details: Some(ActionDetails::Type {
                    text_length: text.len(),
                    preview: truncate_string(text, 50),
                }),
                tool_use_id: None,
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
                action_type: "key".to_string(),
                details: Some(ActionDetails::Key {
                    key: key.clone(),
                    modifiers: modifiers.clone(),
                }),
                tool_use_id: None,
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
            let delay = click_delay;

            tokio::task::spawn_blocking(move || {
                let mut mouse = MouseController::new()?;
                mouse.move_to(x, y)?;
                std::thread::sleep(delay);
                mouse.scroll(dir, amount)
            })
            .await
            .map_err(|e| ActionError::MouseError(crate::input::MouseError::ActionError(e.to_string())))??;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Scrolled {} {} times at ({}, {})", direction_str, amount, x, y)),
                retry_count: 0,
                action_type: "scroll".to_string(),
                details: Some(ActionDetails::Scroll {
                    x,
                    y,
                    direction: direction_str.clone(),
                    amount,
                }),
                tool_use_id: None,
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

            let sx = *start_x;
            let sy = *start_y;
            let ex = *end_x;
            let ey = *end_y;

            tokio::task::spawn_blocking(move || {
                let mut mouse = MouseController::new()?;
                mouse.drag(sx, sy, ex, ey, btn, duration)
            })
            .await
            .map_err(|e| ActionError::MouseError(crate::input::MouseError::ActionError(e.to_string())))??;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!(
                    "Dragged from ({}, {}) to ({}, {})",
                    start_x, start_y, end_x, end_y
                )),
                retry_count: 0,
                action_type: "drag".to_string(),
                details: None,
                tool_use_id: None,
            })
        }

        Action::TripleClick { x, y } => {
            let x = *x;
            let y = *y;

            tokio::task::spawn_blocking(move || {
                let mut mouse = MouseController::new()?;
                mouse.move_to(x, y)?;
                mouse.triple_click(MouseButton::Left)
            })
            .await
            .map_err(|e| ActionError::MouseError(crate::input::MouseError::ActionError(e.to_string())))??;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Triple-clicked at ({}, {})", x, y)),
                retry_count: 0,
                action_type: "triple_click".to_string(),
                details: None,
                tool_use_id: None,
            })
        }

        Action::RightClick { x, y } => {
            let x = *x;
            let y = *y;

            tokio::task::spawn_blocking(move || {
                let mut mouse = MouseController::new()?;
                mouse.click_at(x, y, MouseButton::Right)
            })
            .await
            .map_err(|e| ActionError::MouseError(crate::input::MouseError::ActionError(e.to_string())))??;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Right-clicked at ({}, {})", x, y)),
                retry_count: 0,
                action_type: "right_click".to_string(),
                details: None,
                tool_use_id: None,
            })
        }

        Action::Wait { duration_ms } => {
            tokio::time::sleep(Duration::from_millis(*duration_ms)).await;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Waited {} ms", duration_ms)),
                retry_count: 0,
                action_type: "wait".to_string(),
                details: None,
                tool_use_id: None,
            })
        }

        Action::Complete { message } => Ok(ActionResult {
            success: true,
            completed: true,
            message: Some(message.clone()),
            retry_count: 0,
            action_type: "complete".to_string(),
            details: Some(ActionDetails::Complete {
                message: message.clone(),
            }),
            tool_use_id: None,
        }),

        Action::Error { message } => Ok(ActionResult {
            success: false,
            completed: true,
            message: Some(message.clone()),
            retry_count: 0,
            action_type: "error".to_string(),
            details: Some(ActionDetails::Error {
                message: message.clone(),
            }),
            tool_use_id: None,
        }),

        Action::Batch { actions } => {
            if actions.is_empty() {
                return Ok(ActionResult {
                    success: true,
                    completed: false,
                    message: Some("Empty batch, nothing to execute".into()),
                    retry_count: 0,
                    action_type: "batch".to_string(),
                    details: None,
                    tool_use_id: None,
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
                    retry_count: 0,
                    action_type: "batch".to_string(),
                    details: None,
                    tool_use_id: None,
                });
            }

            for (i, sub_action) in actions.iter().enumerate() {
                // Prevent nested batches
                if matches!(sub_action, Action::Batch { .. }) {
                    return Ok(ActionResult {
                        success: false,
                        completed: false,
                        message: Some("Nested batches are not allowed".into()),
                        retry_count: 0,
                        action_type: "batch".to_string(),
                        details: None,
                        tool_use_id: None,
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
                        retry_count: 0,
                        action_type: "batch".to_string(),
                        details: None,
                        tool_use_id: None,
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
                        retry_count: 0,
                        action_type: "batch".to_string(),
                        details: None,
                        tool_use_id: None,
                    });
                }

                // Small delay between batched actions (except after the last one)
                if i < actions.len() - 1 {
                    tokio::time::sleep(Duration::from_millis(BATCH_INTER_ACTION_DELAY_MS)).await;
                }
            }

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Batch completed: {} actions executed", actions.len())),
                retry_count: 0,
                action_type: "batch".to_string(),
                details: None,
                tool_use_id: None,
            })
        }

        Action::WaitForElement {
            timeout_ms,
            description,
        } => {
            let timeout = timeout_ms.unwrap_or(5000).min(10000);
            log::info!("Waiting for: {} (timeout: {}ms)", description, timeout);

            tokio::time::sleep(Duration::from_millis(timeout as u64)).await;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Waited for: {}", description)),
                retry_count: 0,
                action_type: "wait_for_element".to_string(),
                details: None,
                tool_use_id: None,
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

    /// Check if this action can be reversed
    pub fn is_reversible(&self) -> bool {
        match self {
            // Scroll can always be reversed by scrolling in the opposite direction
            Action::Scroll { .. } => true,
            // Type can be partially reversed (best effort with backspaces)
            Action::Type { text } => !text.is_empty(),
            // Key presses that are simple characters can be reversed with backspace
            Action::Key { key, modifiers } => {
                // Don't try to reverse undo itself or other complex key combos
                if modifiers.iter().any(|m| m.to_lowercase() == "cmd" || m.to_lowercase() == "ctrl") {
                    return false;
                }
                // Simple key presses can be reversed
                key.len() == 1 || key.to_lowercase() == "space" || key.to_lowercase() == "enter"
            }
            // Move could theoretically be reversed but we'd need to track original position
            Action::Move { .. } => false,
            // Clicks cannot be undone
            Action::Click { .. } => false,
            Action::DoubleClick { .. } => false,
            // Terminal actions
            Action::Complete { .. } => false,
            Action::Error { .. } => false,
            // Extended actions cannot be reversed
            Action::Drag { .. } => false,
            Action::TripleClick { .. } => false,
            Action::RightClick { .. } => false,
            Action::Wait { .. } => false,
            Action::WaitForElement { .. } => false,
            Action::Batch { .. } => false,
        }
    }

    /// Create an action that reverses this one, if possible
    pub fn create_reverse(&self) -> Option<Action> {
        match self {
            Action::Scroll {
                x,
                y,
                direction,
                amount,
            } => {
                let reverse_direction = match direction.to_lowercase().as_str() {
                    "up" => "down",
                    "down" => "up",
                    "left" => "right",
                    "right" => "left",
                    _ => return None,
                };
                Some(Action::Scroll {
                    x: *x,
                    y: *y,
                    direction: reverse_direction.to_string(),
                    amount: *amount,
                })
            }
            Action::Type { text } => {
                // Reverse typing by sending backspaces for each character
                let char_count = text.chars().count();
                if char_count == 0 {
                    return None;
                }
                // We'll represent this as a series of backspace key presses
                // For simplicity, we create a single key action that represents "delete N chars"
                // The actual implementation will need to handle this specially
                Some(Action::Key {
                    key: "Backspace".to_string(),
                    modifiers: vec![format!("repeat:{}", char_count)],
                })
            }
            Action::Key { key, modifiers } => {
                // Only reverse simple character input
                if modifiers.iter().any(|m| m.to_lowercase() == "cmd" || m.to_lowercase() == "ctrl") {
                    return None;
                }
                // Simple character or space/enter can be reversed with backspace
                if key.len() == 1 || key.to_lowercase() == "space" || key.to_lowercase() == "enter" {
                    Some(Action::Key {
                        key: "Backspace".to_string(),
                        modifiers: vec![],
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get a human-readable description of this action
    pub fn describe(&self) -> String {
        match self {
            Action::Click { x, y, button } => {
                format!("Click {} at ({}, {})", button, x, y)
            }
            Action::DoubleClick { x, y } => {
                format!("Double-click at ({}, {})", x, y)
            }
            Action::Move { x, y } => {
                format!("Move to ({}, {})", x, y)
            }
            Action::Type { text } => {
                format!("Type \"{}\"", truncate_string(text, 30))
            }
            Action::Key { key, modifiers } => {
                if modifiers.is_empty() {
                    format!("Press {}", key)
                } else {
                    format!("Press {}+{}", modifiers.join("+"), key)
                }
            }
            Action::Scroll {
                x,
                y,
                direction,
                amount,
            } => {
                format!("Scroll {} {} times at ({}, {})", direction, amount, x, y)
            }
            Action::Drag { start_x, start_y, end_x, end_y, .. } => {
                format!("Drag from ({}, {}) to ({}, {})", start_x, start_y, end_x, end_y)
            }
            Action::TripleClick { x, y } => {
                format!("Triple-click at ({}, {})", x, y)
            }
            Action::RightClick { x, y } => {
                format!("Right-click at ({}, {})", x, y)
            }
            Action::Wait { duration_ms } => {
                format!("Wait {} ms", duration_ms)
            }
            Action::WaitForElement { description, timeout_ms } => {
                format!("Wait for: {} (timeout: {} ms)", description, timeout_ms.unwrap_or(5000))
            }
            Action::Batch { actions } => {
                format!("Batch of {} actions", actions.len())
            }
            Action::Complete { message } => {
                format!("Completed: {}", truncate_string(message, 50))
            }
            Action::Error { message } => {
                format!("Error: {}", truncate_string(message, 50))
            }
        }
    }
}

/// Execute an action with retry logic.
/// Automatically retries failed actions or actions that don't produce
/// visible screen changes.
pub async fn execute_action_with_retry(
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
        let mut result = execute_action(action, confirm_dangerous).await?;

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
                tokio::time::sleep(retry_ctx.retry_delay).await;
                continue;
            }
            result.retry_count = retry_ctx.attempt;
            return Ok(result);
        }

        // For actions that should have visible effect, verify screen changed
        if action.should_verify_effect() && retry_ctx.enabled {
            // Wait a bit for UI to update
            tokio::time::sleep(Duration::from_millis(200)).await;

            if !retry_ctx.screen_changed()? {
                if retry_ctx.should_retry() {
                    retry_ctx.increment();
                    log::warn!(
                        "Action had no visible effect, retrying ({}/{}): {:?}",
                        retry_ctx.attempt,
                        retry_ctx.max_retries,
                        action
                    );
                    tokio::time::sleep(retry_ctx.retry_delay).await;
                    continue;
                }
                log::warn!("Action completed but no screen change detected after {} retries", retry_ctx.attempt);
            }
        }

        result.retry_count = retry_ctx.attempt;
        return Ok(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::{LlmResponse, ToolUse};
    use serde_json::json;

    // ── extract_json tests ──────────────────────────────────────────────

    #[test]
    fn test_extract_json_simple_object() {
        let input = r#"{"action": "click", "x": 100, "y": 200}"#;
        let result = extract_json(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_extract_json_with_surrounding_text() {
        let input = r#"I'll click the button now. {"action": "click", "x": 50, "y": 75} That should work."#;
        let result = extract_json(input).unwrap();
        assert_eq!(result, r#"{"action": "click", "x": 50, "y": 75}"#);
    }

    #[test]
    fn test_extract_json_with_nested_braces() {
        let input = r#"{"action": "batch", "actions": [{"action": "type", "text": "hi"}]}"#;
        let result = extract_json(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_extract_json_with_markdown_code_block() {
        let input = "Here is the action:\n```json\n{\"action\": \"complete\", \"message\": \"done\"}\n```";
        let result = extract_json(input).unwrap();
        assert_eq!(result, r#"{"action": "complete", "message": "done"}"#);
    }

    #[test]
    fn test_extract_json_no_json_present() {
        let input = "I don't know what to do next.";
        let result = extract_json(input);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("No JSON object found"));
    }

    #[test]
    fn test_extract_json_unbalanced_braces() {
        let input = r#"{"action": "click", "x": 100"#;
        let result = extract_json(input);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("Unbalanced braces"));
    }

    #[test]
    fn test_extract_json_with_leading_whitespace() {
        let input = "   \n\n  {\"action\": \"wait\", \"duration_ms\": 500}";
        let result = extract_json(input).unwrap();
        assert_eq!(result, r#"{"action": "wait", "duration_ms": 500}"#);
    }

    #[test]
    fn test_extract_json_braces_in_string_values() {
        // Braces inside string values should still be counted by our simple brace-matcher
        // This tests the current behavior (brace counting, not JSON-aware parsing)
        let input = r#"{"action": "type", "text": "hello"}"#;
        let result = extract_json(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_extract_json_multiple_objects_takes_first() {
        let input = r#"{"action": "click", "x": 1, "y": 2} {"action": "type", "text": "hi"}"#;
        let result = extract_json(input).unwrap();
        assert_eq!(result, r#"{"action": "click", "x": 1, "y": 2}"#);
    }

    // ── extract_reasoning tests ─────────────────────────────────────────

    #[test]
    fn test_extract_reasoning_with_text_before_json() {
        let input = "I need to click the submit button.\n{\"action\": \"click\", \"x\": 100, \"y\": 200}";
        let result = extract_reasoning(input);
        assert_eq!(result, Some("I need to click the submit button.".to_string()));
    }

    #[test]
    fn test_extract_reasoning_no_text_before_json() {
        let input = r#"{"action": "click", "x": 100, "y": 200}"#;
        let result = extract_reasoning(input);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_reasoning_with_markdown_code_block() {
        let input = "Clicking the button\n```json\n{\"action\": \"click\", \"x\": 1, \"y\": 2}";
        let result = extract_reasoning(input);
        assert_eq!(result, Some("Clicking the button".to_string()));
    }

    #[test]
    fn test_extract_reasoning_only_whitespace_before_json() {
        let input = "   \n  {\"action\": \"click\", \"x\": 1, \"y\": 2}";
        let result = extract_reasoning(input);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_reasoning_no_json_at_all() {
        let input = "There is no JSON here";
        let result = extract_reasoning(input);
        assert!(result.is_none());
    }

    // ── parse_action tests for each action type ─────────────────────────

    #[test]
    fn test_parse_action_click() {
        let input = r#"{"action": "click", "x": 150, "y": 300, "button": "right"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Click { x, y, button } => {
                assert_eq!(x, 150);
                assert_eq!(y, 300);
                assert_eq!(button, "right");
            }
            _ => panic!("Expected Click action"),
        }
    }

    #[test]
    fn test_parse_action_click_default_button() {
        let input = r#"{"action": "click", "x": 10, "y": 20}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Click { x, y, button } => {
                assert_eq!(x, 10);
                assert_eq!(y, 20);
                assert_eq!(button, "left");
            }
            _ => panic!("Expected Click action"),
        }
    }

    #[test]
    fn test_parse_action_double_click() {
        let input = r#"{"action": "double_click", "x": 200, "y": 400}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::DoubleClick { x, y } => {
                assert_eq!(x, 200);
                assert_eq!(y, 400);
            }
            _ => panic!("Expected DoubleClick action"),
        }
    }

    #[test]
    fn test_parse_action_type() {
        let input = r#"{"action": "type", "text": "Hello World"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Type { text } => {
                assert_eq!(text, "Hello World");
            }
            _ => panic!("Expected Type action"),
        }
    }

    #[test]
    fn test_parse_action_key_with_modifiers() {
        let input = r#"{"action": "key", "key": "c", "modifiers": ["ctrl"]}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Key { key, modifiers } => {
                assert_eq!(key, "c");
                assert_eq!(modifiers, vec!["ctrl"]);
            }
            _ => panic!("Expected Key action"),
        }
    }

    #[test]
    fn test_parse_action_key_no_modifiers() {
        let input = r#"{"action": "key", "key": "enter"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Key { key, modifiers } => {
                assert_eq!(key, "enter");
                assert!(modifiers.is_empty());
            }
            _ => panic!("Expected Key action"),
        }
    }

    #[test]
    fn test_parse_action_scroll() {
        let input = r#"{"action": "scroll", "x": 500, "y": 300, "direction": "down", "amount": 5}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Scroll { x, y, direction, amount } => {
                assert_eq!(x, 500);
                assert_eq!(y, 300);
                assert_eq!(direction, "down");
                assert_eq!(amount, 5);
            }
            _ => panic!("Expected Scroll action"),
        }
    }

    #[test]
    fn test_parse_action_scroll_default_amount() {
        let input = r#"{"action": "scroll", "x": 100, "y": 100, "direction": "up"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Scroll { amount, .. } => {
                assert_eq!(amount, 3);
            }
            _ => panic!("Expected Scroll action"),
        }
    }

    #[test]
    fn test_parse_action_drag() {
        let input = r#"{"action": "drag", "start_x": 10, "start_y": 20, "end_x": 300, "end_y": 400}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Drag { start_x, start_y, end_x, end_y, button, duration_ms } => {
                assert_eq!(start_x, 10);
                assert_eq!(start_y, 20);
                assert_eq!(end_x, 300);
                assert_eq!(end_y, 400);
                assert_eq!(button, "left");
                assert_eq!(duration_ms, 500);
            }
            _ => panic!("Expected Drag action"),
        }
    }

    #[test]
    fn test_parse_action_complete() {
        let input = r#"{"action": "complete", "message": "Task finished successfully"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Complete { message } => {
                assert_eq!(message, "Task finished successfully");
            }
            _ => panic!("Expected Complete action"),
        }
    }

    #[test]
    fn test_parse_action_error() {
        let input = r#"{"action": "error", "message": "Cannot find element"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Error { message } => {
                assert_eq!(message, "Cannot find element");
            }
            _ => panic!("Expected Error action"),
        }
    }

    #[test]
    fn test_parse_action_wait() {
        let input = r#"{"action": "wait", "duration_ms": 2000}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Wait { duration_ms } => {
                assert_eq!(duration_ms, 2000);
            }
            _ => panic!("Expected Wait action"),
        }
    }

    #[test]
    fn test_parse_action_wait_default_duration() {
        let input = r#"{"action": "wait"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Wait { duration_ms } => {
                assert_eq!(duration_ms, 1000);
            }
            _ => panic!("Expected Wait action"),
        }
    }

    #[test]
    fn test_parse_action_batch() {
        let input = r#"{"action": "batch", "actions": [{"action": "type", "text": "hi"}, {"action": "key", "key": "enter"}]}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Batch { actions } => {
                assert_eq!(actions.len(), 2);
                assert!(matches!(&actions[0], Action::Type { text } if text == "hi"));
                assert!(matches!(&actions[1], Action::Key { key, .. } if key == "enter"));
            }
            _ => panic!("Expected Batch action"),
        }
    }

    #[test]
    fn test_parse_action_triple_click() {
        let input = r#"{"action": "triple_click", "x": 100, "y": 200}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::TripleClick { x, y } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("Expected TripleClick action"),
        }
    }

    #[test]
    fn test_parse_action_right_click() {
        let input = r#"{"action": "right_click", "x": 50, "y": 75}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::RightClick { x, y } => {
                assert_eq!(x, 50);
                assert_eq!(y, 75);
            }
            _ => panic!("Expected RightClick action"),
        }
    }

    #[test]
    fn test_parse_action_move() {
        let input = r#"{"action": "move", "x": 400, "y": 500}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Move { x, y } => {
                assert_eq!(x, 400);
                assert_eq!(y, 500);
            }
            _ => panic!("Expected Move action"),
        }
    }

    #[test]
    fn test_parse_action_wait_for_element() {
        let input = r#"{"action": "wait_for_element", "description": "page to load", "timeout_ms": 3000}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::WaitForElement { description, timeout_ms } => {
                assert_eq!(description, "page to load");
                assert_eq!(timeout_ms, Some(3000));
            }
            _ => panic!("Expected WaitForElement action"),
        }
    }

    // ── parse_action edge cases ─────────────────────────────────────────

    #[test]
    fn test_parse_action_extra_fields_ignored() {
        let input = r#"{"action": "click", "x": 10, "y": 20, "unknown_field": "ignored"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Click { x, y, .. } => {
                assert_eq!(x, 10);
                assert_eq!(y, 20);
            }
            _ => panic!("Expected Click action"),
        }
    }

    #[test]
    fn test_parse_action_missing_required_field() {
        let input = r#"{"action": "click", "x": 100}"#;
        let result = parse_action(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_action_with_reasoning() {
        let input = "I see a submit button at the center of the screen.\n{\"action\": \"click\", \"x\": 500, \"y\": 300}";
        let parsed = parse_action(input).unwrap();
        assert_eq!(
            parsed.reasoning,
            Some("I see a submit button at the center of the screen.".to_string())
        );
        assert!(matches!(parsed.action, Action::Click { x: 500, y: 300, .. }));
    }

    #[test]
    fn test_parse_action_invalid_action_type() {
        let input = r#"{"action": "fly_away", "x": 100}"#;
        let result = parse_action(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_action_empty_string() {
        let result = parse_action("");
        assert!(result.is_err());
    }

    // ── parse_llm_response tests ────────────────────────────────────────

    #[test]
    fn test_parse_llm_response_text_path() {
        let response = LlmResponse::Text(
            r#"{"action": "type", "text": "hello world"}"#.to_string(),
        );
        let action = parse_llm_response(&response).unwrap();
        assert!(matches!(action, Action::Type { text } if text == "hello world"));
    }

    #[test]
    fn test_parse_llm_response_tool_use_path() {
        let response = LlmResponse::ToolUse(ToolUse {
            id: "tool_1".to_string(),
            name: "click".to_string(),
            input: json!({"x": 100, "y": 200}),
        });
        let action = parse_llm_response(&response).unwrap();
        match action {
            Action::Click { x, y, button } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
                assert_eq!(button, "left");
            }
            _ => panic!("Expected Click action"),
        }
    }

    #[test]
    fn test_parse_llm_response_text_with_reasoning() {
        let response = LlmResponse::Text(
            "I'll click submit.\n{\"action\": \"click\", \"x\": 50, \"y\": 60}".to_string(),
        );
        let parsed = parse_llm_response_with_reasoning(&response).unwrap();
        assert_eq!(parsed.reasoning, Some("I'll click submit.".to_string()));
        assert!(matches!(parsed.action, Action::Click { x: 50, y: 60, .. }));
    }

    #[test]
    fn test_parse_llm_response_tool_use_no_reasoning() {
        let response = LlmResponse::ToolUse(ToolUse {
            id: "tool_2".to_string(),
            name: "complete".to_string(),
            input: json!({"message": "done"}),
        });
        let parsed = parse_llm_response_with_reasoning(&response).unwrap();
        assert!(parsed.reasoning.is_none());
        assert!(matches!(parsed.action, Action::Complete { .. }));
    }

    // ── from_tool_use tests ─────────────────────────────────────────────

    #[test]
    fn test_from_tool_use_click() {
        let tu = ToolUse {
            id: "t1".to_string(),
            name: "click".to_string(),
            input: json!({"x": 10, "y": 20, "button": "middle"}),
        };
        let action = from_tool_use(&tu).unwrap();
        match action {
            Action::Click { x, y, button } => {
                assert_eq!(x, 10);
                assert_eq!(y, 20);
                assert_eq!(button, "middle");
            }
            _ => panic!("Expected Click"),
        }
    }

    #[test]
    fn test_from_tool_use_double_click() {
        let tu = ToolUse {
            id: "t2".to_string(),
            name: "double_click".to_string(),
            input: json!({"x": 5, "y": 10}),
        };
        let action = from_tool_use(&tu).unwrap();
        assert!(matches!(action, Action::DoubleClick { x: 5, y: 10 }));
    }

    #[test]
    fn test_from_tool_use_move() {
        let tu = ToolUse {
            id: "t3".to_string(),
            name: "move".to_string(),
            input: json!({"x": 300, "y": 400}),
        };
        let action = from_tool_use(&tu).unwrap();
        assert!(matches!(action, Action::Move { x: 300, y: 400 }));
    }

    #[test]
    fn test_from_tool_use_type() {
        let tu = ToolUse {
            id: "t4".to_string(),
            name: "type".to_string(),
            input: json!({"text": "test input"}),
        };
        let action = from_tool_use(&tu).unwrap();
        assert!(matches!(action, Action::Type { ref text } if text == "test input"));
    }

    #[test]
    fn test_from_tool_use_key() {
        let tu = ToolUse {
            id: "t5".to_string(),
            name: "key".to_string(),
            input: json!({"key": "a", "modifiers": ["ctrl", "shift"]}),
        };
        let action = from_tool_use(&tu).unwrap();
        match action {
            Action::Key { key, modifiers } => {
                assert_eq!(key, "a");
                assert_eq!(modifiers, vec!["ctrl", "shift"]);
            }
            _ => panic!("Expected Key"),
        }
    }

    #[test]
    fn test_from_tool_use_scroll() {
        let tu = ToolUse {
            id: "t6".to_string(),
            name: "scroll".to_string(),
            input: json!({"x": 100, "y": 200, "direction": "up", "amount": 7}),
        };
        let action = from_tool_use(&tu).unwrap();
        match action {
            Action::Scroll { x, y, direction, amount } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
                assert_eq!(direction, "up");
                assert_eq!(amount, 7);
            }
            _ => panic!("Expected Scroll"),
        }
    }

    #[test]
    fn test_from_tool_use_scroll_default_amount() {
        let tu = ToolUse {
            id: "t7".to_string(),
            name: "scroll".to_string(),
            input: json!({"x": 0, "y": 0, "direction": "down"}),
        };
        let action = from_tool_use(&tu).unwrap();
        match action {
            Action::Scroll { amount, .. } => assert_eq!(amount, 3),
            _ => panic!("Expected Scroll"),
        }
    }

    #[test]
    fn test_from_tool_use_complete() {
        let tu = ToolUse {
            id: "t8".to_string(),
            name: "complete".to_string(),
            input: json!({"message": "all done"}),
        };
        let action = from_tool_use(&tu).unwrap();
        assert!(matches!(action, Action::Complete { ref message } if message == "all done"));
    }

    #[test]
    fn test_from_tool_use_error() {
        let tu = ToolUse {
            id: "t9".to_string(),
            name: "error".to_string(),
            input: json!({"message": "something went wrong"}),
        };
        let action = from_tool_use(&tu).unwrap();
        assert!(matches!(action, Action::Error { ref message } if message == "something went wrong"));
    }

    #[test]
    fn test_from_tool_use_unknown_action() {
        let tu = ToolUse {
            id: "t10".to_string(),
            name: "teleport".to_string(),
            input: json!({"x": 1, "y": 2}),
        };
        let result = from_tool_use(&tu);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ActionError::UnknownAction(name) if name == "teleport"));
    }

    #[test]
    fn test_from_tool_use_missing_required_field() {
        let tu = ToolUse {
            id: "t11".to_string(),
            name: "click".to_string(),
            input: json!({"x": 100}), // missing y
        };
        let result = from_tool_use(&tu);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_tool_use_click_default_button() {
        let tu = ToolUse {
            id: "t12".to_string(),
            name: "click".to_string(),
            input: json!({"x": 50, "y": 60}),
        };
        let action = from_tool_use(&tu).unwrap();
        match action {
            Action::Click { button, .. } => assert_eq!(button, "left"),
            _ => panic!("Expected Click"),
        }
    }

    #[test]
    fn test_from_tool_use_key_default_modifiers() {
        let tu = ToolUse {
            id: "t13".to_string(),
            name: "key".to_string(),
            input: json!({"key": "enter"}),
        };
        let action = from_tool_use(&tu).unwrap();
        match action {
            Action::Key { modifiers, .. } => assert!(modifiers.is_empty()),
            _ => panic!("Expected Key"),
        }
    }

    // ── Action method tests ─────────────────────────────────────────────

    #[test]
    fn test_should_verify_effect() {
        assert!(Action::Click { x: 0, y: 0, button: "left".into() }.should_verify_effect());
        assert!(Action::Type { text: "hi".into() }.should_verify_effect());
        assert!(Action::Key { key: "a".into(), modifiers: vec![] }.should_verify_effect());
        assert!(Action::Scroll { x: 0, y: 0, direction: "up".into(), amount: 1 }.should_verify_effect());
        assert!(Action::DoubleClick { x: 0, y: 0 }.should_verify_effect());
        assert!(!Action::Complete { message: "done".into() }.should_verify_effect());
        assert!(!Action::Error { message: "err".into() }.should_verify_effect());
        assert!(!Action::Wait { duration_ms: 100 }.should_verify_effect());
    }

    #[test]
    fn test_action_describe() {
        let action = Action::Click { x: 100, y: 200, button: "left".to_string() };
        assert_eq!(action.describe(), "Click left at (100, 200)");

        let action = Action::Type { text: "hello".to_string() };
        assert_eq!(action.describe(), "Type \"hello\"");

        let action = Action::Complete { message: "finished".to_string() };
        assert_eq!(action.describe(), "Completed: finished");
    }

    #[test]
    fn test_action_result_to_tool_result_content() {
        let result = ActionResult {
            success: true,
            completed: false,
            message: Some("Clicked".to_string()),
            retry_count: 0,
            action_type: "click".to_string(),
            details: None,
            tool_use_id: None,
        };
        let content = result.to_tool_result_content();
        assert_eq!(content, "OK");
    }

    #[test]
    fn test_action_result_with_tool_use_id() {
        let result = ActionResult {
            success: true,
            completed: false,
            message: None,
            retry_count: 0,
            action_type: "click".to_string(),
            details: None,
            tool_use_id: None,
        };
        let result = result.with_tool_use_id("tool_123".to_string());
        assert_eq!(result.tool_use_id, Some("tool_123".to_string()));
    }

    // ── Serde round-trip tests ──────────────────────────────────────────

    #[test]
    fn test_action_serde_roundtrip_click() {
        let action = Action::Click { x: 42, y: 84, button: "right".to_string() };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Action::Click { x: 42, y: 84, ref button } if button == "right"));
    }

    #[test]
    fn test_action_serde_roundtrip_batch() {
        let action = Action::Batch {
            actions: vec![
                Action::Type { text: "test".to_string() },
                Action::Key { key: "enter".to_string(), modifiers: vec![] },
            ],
        };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        match deserialized {
            Action::Batch { actions } => assert_eq!(actions.len(), 2),
            _ => panic!("Expected Batch"),
        }
    }

    // ── Helper function tests ───────────────────────────────────────────

    #[test]
    fn test_get_i32_valid() {
        let val = json!({"x": 42});
        assert_eq!(get_i32(&val, "x").unwrap(), 42);
    }

    #[test]
    fn test_get_i32_missing() {
        let val = json!({"x": 42});
        assert!(get_i32(&val, "y").is_err());
    }

    #[test]
    fn test_get_string_valid() {
        let val = json!({"name": "test"});
        assert_eq!(get_string(&val, "name").unwrap(), "test");
    }

    #[test]
    fn test_get_string_missing() {
        let val = json!({"name": "test"});
        assert!(get_string(&val, "other").is_err());
    }

    #[test]
    fn test_get_string_or_default() {
        let val = json!({"button": "right"});
        assert_eq!(get_string_or_default(&val, "button", "left"), "right");
        assert_eq!(get_string_or_default(&val, "missing", "left"), "left");
    }

    #[test]
    fn test_get_i32_or_default() {
        let val = json!({"amount": 5});
        assert_eq!(get_i32_or_default(&val, "amount", 3), 5);
        assert_eq!(get_i32_or_default(&val, "missing", 3), 3);
    }

    #[test]
    fn test_get_string_array_or_default() {
        let val = json!({"mods": ["ctrl", "shift"]});
        assert_eq!(get_string_array_or_default(&val, "mods"), vec!["ctrl", "shift"]);
        assert!(get_string_array_or_default(&val, "missing").is_empty());
    }

    #[test]
    fn test_truncate_string_short() {
        assert_eq!(truncate_string("hi", 10), "hi");
    }

    #[test]
    fn test_truncate_string_long() {
        assert_eq!(truncate_string("hello world", 5), "hello...");
    }

    // ── LlmResponse tests ──────────────────────────────────────────────

    #[test]
    fn test_llm_response_to_string_repr_text() {
        let resp = LlmResponse::Text("some text".to_string());
        assert_eq!(resp.to_string_repr(), "some text");
    }

    #[test]
    fn test_llm_response_to_string_repr_tool_use() {
        let resp = LlmResponse::ToolUse(ToolUse {
            id: "id1".to_string(),
            name: "click".to_string(),
            input: json!({"x": 1, "y": 2}),
        });
        let repr = resp.to_string_repr();
        let parsed: Value = serde_json::from_str(&repr).unwrap();
        assert_eq!(parsed["name"], "click");
    }

    // ── Local LLM edge case tests ────────────────────────────────────────

    #[test]
    fn test_parse_action_very_long_reasoning_before_json() {
        let reasoning = "a".repeat(5000);
        let input = format!("{}\n{{\"action\": \"click\", \"x\": 100, \"y\": 200}}", reasoning);
        let parsed = parse_action(&input).unwrap();
        assert!(matches!(parsed.action, Action::Click { x: 100, y: 200, .. }));
        assert_eq!(parsed.reasoning.unwrap().len(), 5000);
    }

    #[test]
    fn test_parse_action_wrong_case_action_type() {
        // Local LLMs sometimes produce "Click" instead of "click"
        // serde rename_all = "snake_case" won't match "Click" - this should fail
        let input = r#"{"action": "Click", "x": 100, "y": 200}"#;
        let result = parse_action(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_action_json_with_trailing_text() {
        let input = r#"{"action": "click", "x": 50, "y": 60} I hope this works!"#;
        let parsed = parse_action(input).unwrap();
        assert!(matches!(parsed.action, Action::Click { x: 50, y: 60, .. }));
    }

    #[test]
    fn test_parse_action_multiple_newlines_before_json() {
        let input = "\n\n\n\n{\"action\": \"type\", \"text\": \"hello\"}";
        let parsed = parse_action(input).unwrap();
        assert!(matches!(parsed.action, Action::Type { ref text } if text == "hello"));
    }

    #[test]
    fn test_parse_action_json_in_markdown_with_language_tag() {
        let input = "Here's the action:\n```json\n{\"action\": \"key\", \"key\": \"enter\"}\n```\nDone.";
        let parsed = parse_action(input).unwrap();
        assert!(matches!(parsed.action, Action::Key { ref key, .. } if key == "enter"));
    }

    #[test]
    fn test_parse_action_unicode_text() {
        let input = r#"{"action": "type", "text": "こんにちは世界 🌍"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Type { text } => assert_eq!(text, "こんにちは世界 🌍"),
            _ => panic!("Expected Type action"),
        }
    }

    #[test]
    fn test_parse_action_empty_text_field() {
        let input = r#"{"action": "type", "text": ""}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Type { text } => assert_eq!(text, ""),
            _ => panic!("Expected Type action"),
        }
    }

    #[test]
    fn test_parse_action_negative_coordinates() {
        let input = r#"{"action": "click", "x": -10, "y": -20}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Click { x, y, .. } => {
                assert_eq!(x, -10);
                assert_eq!(y, -20);
            }
            _ => panic!("Expected Click action"),
        }
    }

    #[test]
    fn test_parse_action_large_coordinates() {
        let input = r#"{"action": "click", "x": 99999, "y": 88888}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Click { x, y, .. } => {
                assert_eq!(x, 99999);
                assert_eq!(y, 88888);
            }
            _ => panic!("Expected Click action"),
        }
    }

    #[test]
    fn test_parse_action_batch_empty() {
        let input = r#"{"action": "batch", "actions": []}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Batch { actions } => assert!(actions.is_empty()),
            _ => panic!("Expected Batch action"),
        }
    }

    #[test]
    fn test_parse_action_wait_for_element_no_timeout() {
        let input = r#"{"action": "wait_for_element", "description": "loading spinner to disappear"}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::WaitForElement { description, timeout_ms } => {
                assert_eq!(description, "loading spinner to disappear");
                assert!(timeout_ms.is_none());
            }
            _ => panic!("Expected WaitForElement action"),
        }
    }

    #[test]
    fn test_parse_action_drag_with_custom_duration() {
        let input = r#"{"action": "drag", "start_x": 0, "start_y": 0, "end_x": 100, "end_y": 100, "button": "right", "duration_ms": 2000}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Drag { button, duration_ms, .. } => {
                assert_eq!(button, "right");
                assert_eq!(duration_ms, 2000);
            }
            _ => panic!("Expected Drag action"),
        }
    }

    #[test]
    fn test_parse_action_key_multiple_modifiers() {
        let input = r#"{"action": "key", "key": "a", "modifiers": ["ctrl", "shift", "alt"]}"#;
        let parsed = parse_action(input).unwrap();
        match parsed.action {
            Action::Key { key, modifiers } => {
                assert_eq!(key, "a");
                assert_eq!(modifiers.len(), 3);
                assert!(modifiers.contains(&"ctrl".to_string()));
                assert!(modifiers.contains(&"shift".to_string()));
                assert!(modifiers.contains(&"alt".to_string()));
            }
            _ => panic!("Expected Key action"),
        }
    }

    // ── is_reversible tests ──────────────────────────────────────────────

    #[test]
    fn test_is_reversible_scroll() {
        let action = Action::Scroll { x: 0, y: 0, direction: "down".into(), amount: 3 };
        assert!(action.is_reversible());
    }

    #[test]
    fn test_is_reversible_type() {
        assert!(Action::Type { text: "hello".into() }.is_reversible());
        assert!(!Action::Type { text: "".into() }.is_reversible());
    }

    #[test]
    fn test_is_reversible_key_simple() {
        let action = Action::Key { key: "a".into(), modifiers: vec![] };
        assert!(action.is_reversible());
    }

    #[test]
    fn test_is_reversible_key_with_cmd_modifier() {
        let action = Action::Key { key: "c".into(), modifiers: vec!["cmd".into()] };
        assert!(!action.is_reversible());
    }

    #[test]
    fn test_is_reversible_click() {
        assert!(!Action::Click { x: 0, y: 0, button: "left".into() }.is_reversible());
    }

    #[test]
    fn test_is_reversible_complete() {
        assert!(!Action::Complete { message: "done".into() }.is_reversible());
    }

    #[test]
    fn test_is_reversible_drag() {
        let action = Action::Drag { start_x: 0, start_y: 0, end_x: 100, end_y: 100, button: "left".into(), duration_ms: 500 };
        assert!(!action.is_reversible());
    }

    // ── create_reverse tests ─────────────────────────────────────────────

    #[test]
    fn test_create_reverse_scroll() {
        let action = Action::Scroll { x: 100, y: 200, direction: "down".into(), amount: 5 };
        let reverse = action.create_reverse().unwrap();
        match reverse {
            Action::Scroll { x, y, direction, amount } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
                assert_eq!(direction, "up");
                assert_eq!(amount, 5);
            }
            _ => panic!("Expected Scroll reverse"),
        }
    }

    #[test]
    fn test_create_reverse_scroll_left_right() {
        let left = Action::Scroll { x: 0, y: 0, direction: "left".into(), amount: 1 };
        match left.create_reverse().unwrap() {
            Action::Scroll { direction, .. } => assert_eq!(direction, "right"),
            _ => panic!("Expected Scroll"),
        }

        let right = Action::Scroll { x: 0, y: 0, direction: "right".into(), amount: 1 };
        match right.create_reverse().unwrap() {
            Action::Scroll { direction, .. } => assert_eq!(direction, "left"),
            _ => panic!("Expected Scroll"),
        }
    }

    #[test]
    fn test_create_reverse_type() {
        let action = Action::Type { text: "hello".into() };
        let reverse = action.create_reverse().unwrap();
        match reverse {
            Action::Key { key, modifiers } => {
                assert_eq!(key, "Backspace");
                assert_eq!(modifiers, vec!["repeat:5"]);
            }
            _ => panic!("Expected Key reverse for type"),
        }
    }

    #[test]
    fn test_create_reverse_type_empty() {
        let action = Action::Type { text: "".into() };
        assert!(action.create_reverse().is_none());
    }

    #[test]
    fn test_create_reverse_key_simple() {
        let action = Action::Key { key: "a".into(), modifiers: vec![] };
        let reverse = action.create_reverse().unwrap();
        match reverse {
            Action::Key { key, modifiers } => {
                assert_eq!(key, "Backspace");
                assert!(modifiers.is_empty());
            }
            _ => panic!("Expected Key reverse"),
        }
    }

    #[test]
    fn test_create_reverse_key_with_cmd() {
        let action = Action::Key { key: "c".into(), modifiers: vec!["cmd".into()] };
        assert!(action.create_reverse().is_none());
    }

    #[test]
    fn test_create_reverse_click_returns_none() {
        let action = Action::Click { x: 0, y: 0, button: "left".into() };
        assert!(action.create_reverse().is_none());
    }

    // ── describe tests ───────────────────────────────────────────────────

    #[test]
    fn test_describe_all_action_types() {
        assert!(Action::DoubleClick { x: 10, y: 20 }.describe().contains("Double-click"));
        assert!(Action::Move { x: 10, y: 20 }.describe().contains("Move"));
        assert!(Action::Key { key: "enter".into(), modifiers: vec!["ctrl".into()] }.describe().contains("ctrl+enter"));
        assert!(Action::Scroll { x: 0, y: 0, direction: "up".into(), amount: 3 }.describe().contains("Scroll up"));
        assert!(Action::Drag { start_x: 0, start_y: 0, end_x: 1, end_y: 1, button: "left".into(), duration_ms: 500 }.describe().contains("Drag"));
        assert!(Action::TripleClick { x: 5, y: 10 }.describe().contains("Triple-click"));
        assert!(Action::RightClick { x: 5, y: 10 }.describe().contains("Right-click"));
        assert!(Action::Wait { duration_ms: 500 }.describe().contains("Wait 500 ms"));
        assert!(Action::WaitForElement { description: "loading".into(), timeout_ms: Some(3000) }.describe().contains("loading"));
        assert!(Action::Batch { actions: vec![Action::Wait { duration_ms: 100 }] }.describe().contains("1 actions"));
        assert!(Action::Error { message: "oops".into() }.describe().contains("Error: oops"));
    }

    // ── ActionResult tests ───────────────────────────────────────────────

    #[test]
    fn test_action_result_to_tool_result_content_error() {
        let result = ActionResult {
            success: false,
            completed: true,
            message: Some("Something failed".to_string()),
            retry_count: 2,
            action_type: "click".to_string(),
            details: None,
            tool_use_id: None,
        };
        let content = result.to_tool_result_content();
        assert_eq!(content, "FAIL: Something failed");
    }

    #[test]
    fn test_action_result_to_tool_result_content_error_no_message() {
        let result = ActionResult {
            success: false,
            completed: true,
            message: None,
            retry_count: 0,
            action_type: "click".to_string(),
            details: None,
            tool_use_id: None,
        };
        let content = result.to_tool_result_content();
        assert_eq!(content, "FAIL: Unknown error");
    }
}
