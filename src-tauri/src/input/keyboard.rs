use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::cell::RefCell;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeyboardError {
    #[error("Failed to create keyboard controller: {0}")]
    InitError(String),
    #[error("Failed to execute keyboard action: {0}")]
    ActionError(String),
    #[error("Unknown key: {0}")]
    UnknownKey(String),
}

thread_local! {
    static THREAD_KEYBOARD_ENIGO: RefCell<Option<Enigo>> = RefCell::new(None);
}

pub struct KeyboardController {
    enigo: Option<Enigo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta, // Cmd on macOS, Win on Windows
}

impl KeyboardController {
    pub fn new() -> Result<Self, KeyboardError> {
        let enigo = THREAD_KEYBOARD_ENIGO.with(|cell| {
            let mut opt = cell.borrow_mut();
            if let Some(existing) = opt.take() {
                Ok(existing)
            } else {
                Enigo::new(&Settings::default())
                    .map_err(|e| KeyboardError::InitError(e.to_string()))
            }
        })?;
        Ok(Self { enigo: Some(enigo) })
    }

    fn enigo(&mut self) -> &mut Enigo {
        self.enigo.as_mut().unwrap()
    }

    pub fn type_text(&mut self, text: &str) -> Result<(), KeyboardError> {
        self.enigo()
            .text(text)
            .map_err(|e| KeyboardError::ActionError(e.to_string()))
    }

    pub fn key_press(&mut self, key: Key) -> Result<(), KeyboardError> {
        self.enigo()
            .key(key, Direction::Click)
            .map_err(|e| KeyboardError::ActionError(e.to_string()))
    }

    pub fn key_down(&mut self, key: Key) -> Result<(), KeyboardError> {
        self.enigo()
            .key(key, Direction::Press)
            .map_err(|e| KeyboardError::ActionError(e.to_string()))
    }

    pub fn key_up(&mut self, key: Key) -> Result<(), KeyboardError> {
        self.enigo()
            .key(key, Direction::Release)
            .map_err(|e| KeyboardError::ActionError(e.to_string()))
    }

    pub fn key_with_modifiers(
        &mut self,
        key: &str,
        modifiers: &[Modifier],
    ) -> Result<(), KeyboardError> {
        // Press modifiers
        for modifier in modifiers {
            let mod_key = modifier_to_key(*modifier);
            self.key_down(mod_key)?;
        }

        // Press and release the main key
        let main_key = parse_key(key)?;
        self.key_press(main_key)?;

        // Release modifiers in reverse order
        for modifier in modifiers.iter().rev() {
            let mod_key = modifier_to_key(*modifier);
            self.key_up(mod_key)?;
        }

        Ok(())
    }
}

impl Drop for KeyboardController {
    fn drop(&mut self) {
        if let Some(enigo) = self.enigo.take() {
            THREAD_KEYBOARD_ENIGO.with(|cell| {
                *cell.borrow_mut() = Some(enigo);
            });
        }
    }
}

fn modifier_to_key(modifier: Modifier) -> Key {
    match modifier {
        Modifier::Ctrl => Key::Control,
        Modifier::Alt => Key::Alt,
        Modifier::Shift => Key::Shift,
        Modifier::Meta => Key::Meta,
    }
}

pub fn parse_key(key_str: &str) -> Result<Key, KeyboardError> {
    let key = match key_str.to_lowercase().as_str() {
        // Letters
        "a" => Key::Unicode('a'),
        "b" => Key::Unicode('b'),
        "c" => Key::Unicode('c'),
        "d" => Key::Unicode('d'),
        "e" => Key::Unicode('e'),
        "f" => Key::Unicode('f'),
        "g" => Key::Unicode('g'),
        "h" => Key::Unicode('h'),
        "i" => Key::Unicode('i'),
        "j" => Key::Unicode('j'),
        "k" => Key::Unicode('k'),
        "l" => Key::Unicode('l'),
        "m" => Key::Unicode('m'),
        "n" => Key::Unicode('n'),
        "o" => Key::Unicode('o'),
        "p" => Key::Unicode('p'),
        "q" => Key::Unicode('q'),
        "r" => Key::Unicode('r'),
        "s" => Key::Unicode('s'),
        "t" => Key::Unicode('t'),
        "u" => Key::Unicode('u'),
        "v" => Key::Unicode('v'),
        "w" => Key::Unicode('w'),
        "x" => Key::Unicode('x'),
        "y" => Key::Unicode('y'),
        "z" => Key::Unicode('z'),

        // Numbers
        "0" => Key::Unicode('0'),
        "1" => Key::Unicode('1'),
        "2" => Key::Unicode('2'),
        "3" => Key::Unicode('3'),
        "4" => Key::Unicode('4'),
        "5" => Key::Unicode('5'),
        "6" => Key::Unicode('6'),
        "7" => Key::Unicode('7'),
        "8" => Key::Unicode('8'),
        "9" => Key::Unicode('9'),

        // Special keys
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "escape" | "esc" => Key::Escape,
        "up" | "arrowup" => Key::UpArrow,
        "down" | "arrowdown" => Key::DownArrow,
        "left" | "arrowleft" => Key::LeftArrow,
        "right" | "arrowright" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,

        // Function keys
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,

        // Modifiers (for direct key press)
        "ctrl" | "control" => Key::Control,
        "alt" | "option" => Key::Alt,
        "shift" => Key::Shift,
        "meta" | "cmd" | "command" | "win" | "super" => Key::Meta,

        _ => return Err(KeyboardError::UnknownKey(key_str.to_string())),
    };

    Ok(key)
}

pub fn parse_modifier(modifier_str: &str) -> Option<Modifier> {
    match modifier_str.to_lowercase().as_str() {
        "ctrl" | "control" => Some(Modifier::Ctrl),
        "alt" | "option" => Some(Modifier::Alt),
        "shift" => Some(Modifier::Shift),
        "meta" | "cmd" | "command" | "win" | "super" => Some(Modifier::Meta),
        _ => None,
    }
}

pub fn is_dangerous_key_combination(key: &str, modifiers: &[Modifier]) -> bool {
    let key_lower = key.to_lowercase();

    // Delete with modifiers
    if (key_lower == "delete" || key_lower == "backspace") && modifiers.contains(&Modifier::Meta) {
        return true;
    }

    // Close window (Cmd+W on macOS, Alt+F4 on Windows)
    if key_lower == "w" && modifiers.contains(&Modifier::Meta) {
        return true;
    }
    if key_lower == "f4" && modifiers.contains(&Modifier::Alt) {
        return true;
    }

    // Quit application (Cmd+Q)
    if key_lower == "q" && modifiers.contains(&Modifier::Meta) {
        return true;
    }

    // Force quit (Cmd+Shift+Q on macOS)
    if key_lower == "q"
        && modifiers.contains(&Modifier::Meta)
        && modifiers.contains(&Modifier::Shift)
    {
        return true;
    }

    // Clear browsing data (Cmd+Shift+Delete / Ctrl+Shift+Delete)
    if (key_lower == "delete" || key_lower == "backspace")
        && modifiers.contains(&Modifier::Shift)
        && (modifiers.contains(&Modifier::Meta) || modifiers.contains(&Modifier::Ctrl))
    {
        return true;
    }

    // Task manager / Force quit dialog (Ctrl+Alt+Delete / Cmd+Option+Escape)
    if key_lower == "delete"
        && modifiers.contains(&Modifier::Ctrl)
        && modifiers.contains(&Modifier::Alt)
    {
        return true;
    }
    if key_lower == "escape"
        && modifiers.contains(&Modifier::Meta)
        && modifiers.contains(&Modifier::Alt)
    {
        return true;
    }

    false
}
