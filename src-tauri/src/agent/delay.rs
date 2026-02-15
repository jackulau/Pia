#![allow(dead_code)]

use std::time::Duration;

const BASE_ITERATION_DELAY_MS: u64 = 500;
const BASE_CLICK_DELAY_MS: u64 = 50;
const BASE_INDICATOR_PAUSE_MS: u64 = 300;
const BASE_CURSOR_HIDE_MS: u64 = 150;
const BASE_PREVIEW_DELAY_MS: u64 = 500;
const BASE_PARSE_ERROR_DELAY_MS: u64 = 500;
const MIN_MULTIPLIER: f32 = 0.25;
const MAX_MULTIPLIER: f32 = 3.0;

#[derive(Debug, Clone)]
pub struct DelayController {
    speed_multiplier: f32,
}

impl DelayController {
    pub fn new(speed_multiplier: f32) -> Self {
        let clamped = speed_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        Self {
            speed_multiplier: clamped,
        }
    }

    pub fn calculate_delay(&self, base_ms: u64) -> Duration {
        let adjusted_ms = (base_ms as f32 / self.speed_multiplier) as u64;
        Duration::from_millis(adjusted_ms.max(1))
    }

    pub fn iteration_delay(&self) -> Duration {
        self.calculate_delay(BASE_ITERATION_DELAY_MS)
    }

    pub fn click_delay(&self) -> Duration {
        self.calculate_delay(BASE_CLICK_DELAY_MS)
    }

    pub fn indicator_pause(&self) -> Duration {
        self.calculate_delay(BASE_INDICATOR_PAUSE_MS)
    }

    pub fn cursor_hide_delay(&self) -> Duration {
        self.calculate_delay(BASE_CURSOR_HIDE_MS)
    }

    pub fn preview_delay(&self) -> Duration {
        self.calculate_delay(BASE_PREVIEW_DELAY_MS)
    }

    pub fn parse_error_delay(&self) -> Duration {
        self.calculate_delay(BASE_PARSE_ERROR_DELAY_MS)
    }

    pub fn speed_multiplier(&self) -> f32 {
        self.speed_multiplier
    }

    pub fn set_speed_multiplier(&mut self, multiplier: f32) {
        self.speed_multiplier = multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
    }
}

impl Default for DelayController {
    fn default() -> Self {
        Self::new(1.0)
    }
}

pub fn validate_speed_multiplier(multiplier: f32) -> Result<f32, String> {
    if multiplier < MIN_MULTIPLIER || multiplier > MAX_MULTIPLIER {
        Err(format!(
            "Speed multiplier must be between {} and {}",
            MIN_MULTIPLIER, MAX_MULTIPLIER
        ))
    } else {
        Ok(multiplier)
    }
}
