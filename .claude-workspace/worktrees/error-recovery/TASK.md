---
id: error-recovery
name: Implement Error Recovery and Retry Mechanism
wave: 2
priority: 3
dependencies: [conversation-context]
estimated_hours: 4
tags: [backend, reliability]
---

## Objective

Add error recovery and retry mechanisms so the agent doesn't fail completely on transient errors.

## Context

Currently, any error terminates the entire agent loop immediately:
```rust
Err(e) => {
    self.state.set_error(e.to_string()).await;
    return Err(e.into());
}
```

This is too aggressive - many errors are recoverable:
- Network timeouts can be retried
- LLM rate limits should back off and retry
- Invalid JSON responses can ask for clarification
- Screenshot capture failures might succeed on retry

## Implementation

1. Modify `/src-tauri/src/llm/provider.rs`:
   - Add `RetryPolicy` struct with max_retries, backoff settings
   - Create `is_retryable()` method on LlmError
   - Implement exponential backoff helper

2. Modify `/src-tauri/src/agent/loop_runner.rs`:
   - Wrap LLM calls with retry logic
   - Retry screenshot captures (up to 3 times)
   - For parse errors, send error back to LLM for correction
   - Add max_consecutive_errors before giving up
   - Track retry attempts in state

3. Modify `/src-tauri/src/agent/state.rs`:
   - Add `retry_count` and `consecutive_errors` fields
   - Add `Retrying` status

4. Create `/src-tauri/src/agent/recovery.rs`:
   - Centralized retry logic
   - Error classification (retryable vs fatal)
   - Backoff calculation

## Acceptance Criteria

- [ ] Network errors are retried with exponential backoff
- [ ] Rate limits trigger appropriate wait times
- [ ] Parse errors send feedback to LLM
- [ ] Max 3 consecutive errors before failing
- [ ] Retry state is visible in UI
- [ ] Fatal errors still terminate immediately

## Files to Create/Modify

- `src-tauri/src/agent/recovery.rs` - NEW: Recovery utilities
- `src-tauri/src/agent/mod.rs` - Export recovery module
- `src-tauri/src/llm/provider.rs` - Error classification, RetryPolicy
- `src-tauri/src/agent/loop_runner.rs` - Retry logic integration
- `src-tauri/src/agent/state.rs` - Retry state tracking

## Integration Points

- **Provides**: Robust error handling for all operations
- **Consumes**: conversation-context for error feedback
- **Conflicts**: None
