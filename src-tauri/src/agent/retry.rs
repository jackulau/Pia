#![allow(dead_code)]

use crate::capture::{capture_primary_screen, CaptureError, Screenshot};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RetryError {
    #[error("Capture error: {0}")]
    CaptureError(#[from] CaptureError),
}

pub struct RetryContext {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub attempt: u32,
    pub enabled: bool,
    last_screenshot: Option<Screenshot>,
}

impl RetryContext {
    pub fn new(max_retries: u32, retry_delay_ms: u32, enabled: bool) -> Self {
        Self {
            max_retries,
            retry_delay: Duration::from_millis(retry_delay_ms as u64),
            attempt: 0,
            enabled,
            last_screenshot: None,
        }
    }

    pub fn should_retry(&self) -> bool {
        self.enabled && self.attempt < self.max_retries
    }

    pub fn increment(&mut self) {
        self.attempt += 1;
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
        self.last_screenshot = None;
    }

    pub fn capture_before(&mut self) -> Result<(), RetryError> {
        if self.enabled {
            self.last_screenshot = Some(capture_primary_screen()?);
        }
        Ok(())
    }

    pub fn screen_changed(&self) -> Result<bool, RetryError> {
        if !self.enabled {
            return Ok(true);
        }

        if let Some(before) = &self.last_screenshot {
            let after = capture_primary_screen()?;
            Ok(screenshots_differ(before, &after))
        } else {
            Ok(true) // No baseline, assume changed
        }
    }
}

/// Compare two screenshots to detect if the screen changed.
/// This is a simplified implementation that compares base64 encoded images.
/// For performance, it first checks dimensions, then compares the encoded data.
fn screenshots_differ(a: &Screenshot, b: &Screenshot) -> bool {
    // Quick check: different dimensions = different
    if a.width != b.width || a.height != b.height {
        return true;
    }

    // Compare the base64 encoded content
    // This is effective because any pixel change will result in different encoding
    a.base64 != b.base64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_context_should_retry() {
        let mut ctx = RetryContext::new(3, 1000, true);
        assert!(ctx.should_retry());

        ctx.increment();
        assert!(ctx.should_retry());

        ctx.increment();
        assert!(ctx.should_retry());

        ctx.increment();
        assert!(!ctx.should_retry());
    }

    #[test]
    fn test_retry_context_disabled() {
        let ctx = RetryContext::new(3, 1000, false);
        assert!(!ctx.should_retry());
    }

    #[test]
    fn test_retry_context_reset() {
        let mut ctx = RetryContext::new(3, 1000, true);
        ctx.increment();
        ctx.increment();
        assert_eq!(ctx.attempt, 2);

        ctx.reset();
        assert_eq!(ctx.attempt, 0);
        assert!(ctx.should_retry());
    }

    #[test]
    fn test_zero_max_retries_cannot_retry() {
        let ctx = RetryContext::new(0, 100, true);
        assert!(!ctx.should_retry());
    }

    #[test]
    fn test_increment_past_max_no_panic() {
        let mut ctx = RetryContext::new(2, 100, true);
        for _ in 0..10 {
            ctx.increment();
        }
        assert_eq!(ctx.attempt, 10);
        assert!(!ctx.should_retry());
    }

    #[test]
    fn test_reset_restores_can_retry() {
        let mut ctx = RetryContext::new(2, 100, true);
        ctx.increment();
        ctx.increment();
        assert!(!ctx.should_retry());
        ctx.reset();
        assert!(ctx.should_retry());
    }

    #[test]
    fn test_retry_delay_stored() {
        let ctx = RetryContext::new(3, 500, true);
        assert_eq!(ctx.retry_delay, Duration::from_millis(500));
    }

    #[test]
    fn test_screenshots_differ_same() {
        use std::sync::Arc;
        let s = Screenshot {
            width: 100,
            height: 100,
            physical_width: 100,
            physical_height: 100,
            base64: Arc::new("abc123".into()),
        };
        assert!(!screenshots_differ(&s, &s));
    }

    #[test]
    fn test_screenshots_differ_different_size() {
        use std::sync::Arc;
        let a = Screenshot {
            width: 100,
            height: 100,
            physical_width: 100,
            physical_height: 100,
            base64: Arc::new("same".into()),
        };
        let b = Screenshot {
            width: 200,
            height: 100,
            physical_width: 200,
            physical_height: 100,
            base64: Arc::new("same".into()),
        };
        assert!(screenshots_differ(&a, &b));
    }

    #[test]
    fn test_screenshots_differ_different_content() {
        use std::sync::Arc;
        let a = Screenshot {
            width: 100,
            height: 100,
            physical_width: 100,
            physical_height: 100,
            base64: Arc::new("data1".into()),
        };
        let b = Screenshot {
            width: 100,
            height: 100,
            physical_width: 100,
            physical_height: 100,
            base64: Arc::new("data2".into()),
        };
        assert!(screenshots_differ(&a, &b));
    }
}
