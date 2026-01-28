use crate::input::{
    is_dangerous_key_combination, parse_key, parse_modifier, KeyboardController, Modifier,
    MouseButton, MouseController, ScrollDirection,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    Complete {
        message: String,
    },
    Error {
        message: String,
    },
}

fn default_button() -> String {
    "left".to_string()
}

fn default_scroll_amount() -> i32 {
    3
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
    /// The type of action that was executed
    pub action_type: String,
    /// Detailed information about the executed action
    pub details: Option<ActionDetails>,
    /// The tool_use_id this result corresponds to (set by caller)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

impl ActionResult {
    /// Convert this ActionResult to a tool_result content string for the Anthropic API
    pub fn to_tool_result_content(&self) -> String {
        let status = if self.success { "success" } else { "error" };
        let result = json!({
            "status": status,
            "action": self.action_type,
            "message": self.message,
            "details": self.details,
        });
        serde_json::to_string(&result).unwrap_or_else(|_| {
            format!("Action {} {}", self.action_type, status)
        })
    }

    /// Set the tool_use_id for this result
    pub fn with_tool_use_id(mut self, id: String) -> Self {
        self.tool_use_id = Some(id);
        self
    }
}

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

pub fn execute_action(
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

            let mut mouse = MouseController::new()?;
            mouse.click_at(*x, *y, btn)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Clicked {} at ({}, {})", button, x, y)),
                action_type: "click".to_string(),
                details: Some(ActionDetails::Click {
                    x: *x,
                    y: *y,
                    button: button.clone(),
                }),
                tool_use_id: None,
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
                action_type: "double_click".to_string(),
                details: Some(ActionDetails::DoubleClick { x: *x, y: *y }),
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

            let mut mouse = MouseController::new()?;
            mouse.move_to(*x, *y)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            mouse.scroll(dir, *amount)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Scrolled {} {} times at ({}, {})", direction, amount, x, y)),
                action_type: "scroll".to_string(),
                details: Some(ActionDetails::Scroll {
                    x: *x,
                    y: *y,
                    direction: direction.clone(),
                    amount: *amount,
                }),
                tool_use_id: None,
            })
        }

        Action::Complete { message } => Ok(ActionResult {
            success: true,
            completed: true,
            message: Some(message.clone()),
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
            action_type: "error".to_string(),
            details: Some(ActionDetails::Error {
                message: message.clone(),
            }),
            tool_use_id: None,
        }),
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}
