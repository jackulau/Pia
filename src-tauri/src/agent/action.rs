use crate::input::{
    is_dangerous_key_combination, parse_key, parse_modifier, KeyboardController, Modifier,
    MouseButton, MouseController, ScrollDirection,
};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub completed: bool,
    pub message: Option<String>,
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
            })
        }

        Action::Move { x, y } => {
            let mut mouse = MouseController::new()?;
            mouse.move_to(*x, *y)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Moved mouse to ({}, {})", x, y)),
            })
        }

        Action::Type { text } => {
            let mut keyboard = KeyboardController::new()?;
            keyboard.type_text(text)?;

            Ok(ActionResult {
                success: true,
                completed: false,
                message: Some(format!("Typed: {}", truncate_string(text, 50))),
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
            })
        }

        Action::Complete { message } => Ok(ActionResult {
            success: true,
            completed: true,
            message: Some(message.clone()),
        }),

        Action::Error { message } => Ok(ActionResult {
            success: false,
            completed: true,
            message: Some(message.clone()),
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

impl Action {
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
            Action::Complete { message } => {
                format!("Completed: {}", truncate_string(message, 50))
            }
            Action::Error { message } => {
                format!("Error: {}", truncate_string(message, 50))
            }
        }
    }
}
