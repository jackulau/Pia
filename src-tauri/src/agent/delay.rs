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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_multiplier() {
        let dc = DelayController::new(1.0);
        assert_eq!(dc.calculate_delay(500), Duration::from_millis(500));
    }

    #[test]
    fn test_double_speed_halves_delay() {
        let dc = DelayController::new(2.0);
        assert_eq!(dc.calculate_delay(200), Duration::from_millis(100));
    }

    #[test]
    fn test_half_speed_doubles_delay() {
        let dc = DelayController::new(0.5);
        assert_eq!(dc.calculate_delay(100), Duration::from_millis(200));
    }

    #[test]
    fn test_min_clamp() {
        let dc = DelayController::new(0.1);
        assert_eq!(dc.speed_multiplier(), MIN_MULTIPLIER);
    }

    #[test]
    fn test_max_clamp() {
        let dc = DelayController::new(10.0);
        assert_eq!(dc.speed_multiplier(), MAX_MULTIPLIER);
    }

    #[test]
    fn test_negative_clamps_to_min() {
        let dc = DelayController::new(-1.0);
        assert_eq!(dc.speed_multiplier(), MIN_MULTIPLIER);
    }

    #[test]
    fn test_zero_clamps_to_min() {
        let dc = DelayController::new(0.0);
        assert_eq!(dc.speed_multiplier(), MIN_MULTIPLIER);
    }

    #[test]
    fn test_delay_minimum_1ms() {
        let dc = DelayController::new(MAX_MULTIPLIER);
        let delay = dc.calculate_delay(1);
        assert!(delay.as_millis() >= 1);
    }

    #[test]
    fn test_iteration_delay_default() {
        let dc = DelayController::default();
        assert_eq!(dc.iteration_delay(), Duration::from_millis(500));
    }

    #[test]
    fn test_set_speed_multiplier() {
        let mut dc = DelayController::new(1.0);
        dc.set_speed_multiplier(2.0);
        assert_eq!(dc.speed_multiplier(), 2.0);
    }

    #[test]
    fn test_set_speed_multiplier_clamps() {
        let mut dc = DelayController::new(1.0);
        dc.set_speed_multiplier(100.0);
        assert_eq!(dc.speed_multiplier(), MAX_MULTIPLIER);
    }

    #[test]
    fn test_validate_speed_valid() {
        assert!(validate_speed_multiplier(1.0).is_ok());
    }

    #[test]
    fn test_validate_speed_too_low() {
        assert!(validate_speed_multiplier(0.1).is_err());
    }

    #[test]
    fn test_validate_speed_too_high() {
        assert!(validate_speed_multiplier(5.0).is_err());
    }
}
