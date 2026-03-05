use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MouseError {
    #[error("Failed to create mouse controller: {0}")]
    InitError(String),
    #[error("Failed to execute mouse action: {0}")]
    ActionError(String),
}

pub struct MouseController {
    enigo: Enigo,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl From<MouseButton> for Button {
    fn from(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl MouseController {
    pub fn new() -> Result<Self, MouseError> {
        let enigo =
            Enigo::new(&Settings::default()).map_err(|e| MouseError::InitError(e.to_string()))?;
        Ok(Self { enigo })
    }

    pub fn move_to(&mut self, x: i32, y: i32) -> Result<(), MouseError> {
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| MouseError::ActionError(e.to_string()))
    }

    pub fn click(&mut self, button: MouseButton) -> Result<(), MouseError> {
        self.enigo
            .button(button.into(), Direction::Click)
            .map_err(|e| MouseError::ActionError(e.to_string()))
    }

    pub fn double_click(&mut self, button: MouseButton) -> Result<(), MouseError> {
        self.click(button)?;
        self.click(button)?;
        Ok(())
    }

    pub fn mouse_down(&mut self, button: MouseButton) -> Result<(), MouseError> {
        self.enigo
            .button(button.into(), Direction::Press)
            .map_err(|e| MouseError::ActionError(e.to_string()))
    }

    pub fn mouse_up(&mut self, button: MouseButton) -> Result<(), MouseError> {
        self.enigo
            .button(button.into(), Direction::Release)
            .map_err(|e| MouseError::ActionError(e.to_string()))
    }

    pub fn scroll(&mut self, direction: ScrollDirection, amount: i32) -> Result<(), MouseError> {
        match direction {
            ScrollDirection::Up => self
                .enigo
                .scroll(amount, enigo::Axis::Vertical)
                .map_err(|e| MouseError::ActionError(e.to_string())),
            ScrollDirection::Down => self
                .enigo
                .scroll(-amount, enigo::Axis::Vertical)
                .map_err(|e| MouseError::ActionError(e.to_string())),
            ScrollDirection::Left => self
                .enigo
                .scroll(-amount, enigo::Axis::Horizontal)
                .map_err(|e| MouseError::ActionError(e.to_string())),
            ScrollDirection::Right => self
                .enigo
                .scroll(amount, enigo::Axis::Horizontal)
                .map_err(|e| MouseError::ActionError(e.to_string())),
        }
    }

    pub fn click_at(&mut self, x: i32, y: i32, button: MouseButton) -> Result<(), MouseError> {
        self.click_at_with_delay(x, y, button, std::time::Duration::from_millis(50))
    }

    pub fn click_at_with_delay(
        &mut self,
        x: i32,
        y: i32,
        button: MouseButton,
        delay: std::time::Duration,
    ) -> Result<(), MouseError> {
        self.move_to(x, y)?;
        std::thread::sleep(delay);
        self.click(button)
    }

    pub fn triple_click(&mut self, button: MouseButton) -> Result<(), MouseError> {
        self.click(button)?;
        self.click(button)?;
        self.click(button)?;
        Ok(())
    }

    pub fn drag(
        &mut self,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        button: MouseButton,
        duration_ms: u32,
    ) -> Result<(), MouseError> {
        // Move to start position
        self.move_to(start_x, start_y)?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Press button
        self.mouse_down(button)?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Smooth movement to end position (~60fps)
        let steps = (duration_ms / 16).max(5);
        let dx = (end_x - start_x) as f32 / steps as f32;
        let dy = (end_y - start_y) as f32 / steps as f32;

        for i in 1..=steps {
            let x = start_x + (dx * i as f32) as i32;
            let y = start_y + (dy * i as f32) as i32;
            self.move_to(x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        // Ensure we're at exact end position
        self.move_to(end_x, end_y)?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Release button
        self.mouse_up(button)?;

        Ok(())
    }
}
