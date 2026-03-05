#![allow(dead_code)]

use crate::llm::LlmError;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Classification of errors for retry decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClassification {
    /// Error can be retried immediately with backoff
    Retryable,
    /// Rate limit hit - requires longer wait
    RateLimited { wait_seconds: u64 },
    /// Fatal error - should not retry
    Fatal,
}

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Create a policy for LLM API calls
    pub fn for_llm_calls() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        }
    }

    /// Create a policy for screenshot captures
    pub fn for_screenshots() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(2),
            backoff_multiplier: 1.5,
        }
    }

    /// Calculate delay for a given attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32 - 1);
        let delay = Duration::from_millis(delay_ms as u64);

        delay.min(self.max_delay)
    }
}

/// Classify an LLM error for retry decisions
pub fn classify_llm_error(error: &LlmError) -> ErrorClassification {
    match error {
        // Network errors are generally retryable
        LlmError::RequestError(req_err) => {
            if req_err.is_timeout() || req_err.is_connect() {
                ErrorClassification::Retryable
            } else if req_err.is_status() {
                // Check status codes
                if let Some(status) = req_err.status() {
                    match status.as_u16() {
                        429 => {
                            // Rate limited - default to 30 second wait
                            ErrorClassification::RateLimited { wait_seconds: 30 }
                        }
                        500..=599 => ErrorClassification::Retryable,
                        401 | 403 => ErrorClassification::Fatal,
                        _ => ErrorClassification::Fatal,
                    }
                } else {
                    ErrorClassification::Fatal
                }
            } else {
                ErrorClassification::Retryable
            }
        }

        // Parse errors might be retryable if the LLM gave bad output
        LlmError::ParseError(_) => ErrorClassification::Retryable,

        // API errors - check for specific messages
        LlmError::ApiError(msg) => {
            let msg_lower = msg.to_lowercase();
            if msg_lower.contains("rate limit") || msg_lower.contains("too many requests") {
                ErrorClassification::RateLimited { wait_seconds: 60 }
            } else if msg_lower.contains("overloaded") || msg_lower.contains("capacity") {
                ErrorClassification::RateLimited { wait_seconds: 30 }
            } else if msg_lower.contains("timeout") || msg_lower.contains("temporarily") {
                ErrorClassification::Retryable
            } else {
                ErrorClassification::Fatal
            }
        }

        // Stream errors might be recoverable
        LlmError::StreamError(_) => ErrorClassification::Retryable,

        // Not configured is fatal
        LlmError::NotConfigured => ErrorClassification::Fatal,
    }
}

/// Result of a retry operation
#[derive(Debug)]
pub struct RetryResult<T, E> {
    /// The result (success or final error)
    pub result: Result<T, E>,
    /// Number of attempts made
    pub attempts: u32,
    /// Whether we exhausted all retries
    pub exhausted: bool,
}

/// Execute an async operation with retry logic
pub async fn retry_with_policy<T, E, F, Fut>(
    policy: &RetryPolicy,
    classify: impl Fn(&E) -> ErrorClassification,
    mut operation: F,
) -> RetryResult<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempts = 0;

    loop {
        attempts += 1;

        match operation().await {
            Ok(value) => {
                return RetryResult {
                    result: Ok(value),
                    attempts,
                    exhausted: false,
                };
            }
            Err(error) => {
                let classification = classify(&error);

                // Check if we should retry
                let should_retry = match classification {
                    ErrorClassification::Fatal => false,
                    ErrorClassification::Retryable | ErrorClassification::RateLimited { .. } => {
                        attempts <= policy.max_retries
                    }
                };

                if !should_retry {
                    return RetryResult {
                        result: Err(error),
                        attempts,
                        exhausted: matches!(
                            classification,
                            ErrorClassification::Retryable | ErrorClassification::RateLimited { .. }
                        ),
                    };
                }

                // Calculate delay
                let delay = match classification {
                    ErrorClassification::RateLimited { wait_seconds } => {
                        Duration::from_secs(wait_seconds)
                    }
                    _ => policy.delay_for_attempt(attempts),
                };

                sleep(delay).await;
            }
        }
    }
}

/// Classify a capture error for retry decisions
pub fn classify_capture_error(error: &crate::capture::CaptureError) -> ErrorClassification {
    use crate::capture::CaptureError;
    match error {
        // No monitors is fatal - won't change on retry
        CaptureError::NoMonitors => ErrorClassification::Fatal,
        // Transient capture errors are retryable
        CaptureError::CaptureError(_) => ErrorClassification::Retryable,
        // Encoding errors might be transient (memory pressure, etc.)
        CaptureError::EncodeError(_) => ErrorClassification::Retryable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_calculation() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        };

        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(400));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_millis(800));
    }

    #[test]
    fn test_delay_respects_max() {
        let policy = RetryPolicy {
            max_retries: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 3.0,
        };

        // After several attempts, delay should be capped at max
        assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(5));
    }

    #[test]
    fn test_default_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.backoff_multiplier, 2.0);
    }
}
