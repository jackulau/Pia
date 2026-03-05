---
id: self-correction-retry
name: Self-Correction with Retry Logic
wave: 2
priority: 1
dependencies: [wait-for-element]
estimated_hours: 6
tags: [backend, reliability, llm]
---

## Objective

Enable the agent to detect action failures and automatically retry with corrections, significantly increasing success rates for flaky UI interactions.

## Context

Many automation failures are transient: elements haven't loaded, windows are in unexpected positions, or clicks miss their targets by small margins. Currently, any failure terminates the agent. With self-correction, the agent can detect failures via screenshot comparison and retry actions, possibly with adjusted coordinates or timing.

Depends on `wait-for-element` because retry logic should incorporate waiting between attempts.

## Implementation

### 1. Add Retry Configuration (`src-tauri/src/config/settings.rs`)

```rust
pub struct GeneralConfig {
    // ... existing fields
    pub max_retries: u32,           // Default: 3
    pub retry_delay_ms: u32,        // Default: 1000
    pub enable_self_correction: bool, // Default: true
}
```

### 2. Create Retry Module (`src-tauri/src/agent/retry.rs`)

```rust
use crate::capture::screenshot::{capture_primary_screen, Screenshot};

pub struct RetryContext {
    pub max_retries: u32,
    pub retry_delay_ms: u32,
    pub attempt: u32,
    pub last_screenshot: Option<Screenshot>,
}

impl RetryContext {
    pub fn new(max_retries: u32, retry_delay_ms: u32) -> Self {
        Self {
            max_retries,
            retry_delay_ms,
            attempt: 0,
            last_screenshot: None,
        }
    }

    pub fn should_retry(&self) -> bool {
        self.attempt < self.max_retries
    }

    pub fn increment(&mut self) {
        self.attempt += 1;
    }

    pub fn capture_before(&mut self) -> Result<(), CaptureError> {
        self.last_screenshot = Some(capture_primary_screen()?);
        Ok(())
    }

    pub fn screen_changed(&self) -> Result<bool, CaptureError> {
        if let Some(before) = &self.last_screenshot {
            let after = capture_primary_screen()?;
            Ok(screenshots_differ(before, &after))
        } else {
            Ok(true) // No baseline, assume changed
        }
    }
}

/// Simple pixel-based comparison (can be enhanced with smarter algorithms)
fn screenshots_differ(a: &Screenshot, b: &Screenshot) -> bool {
    // Quick check: different dimensions = different
    if a.width != b.width || a.height != b.height {
        return true;
    }

    // Decode and compare a sample of pixels
    // For performance, compare only a subset of regions
    // This is a simplified implementation
    a.base64 != b.base64  // Naive: any change detected
}
```

### 3. Integrate Retry into Action Execution (`src-tauri/src/agent/action.rs`)

```rust
pub async fn execute_action_with_retry(
    action: &Action,
    confirm_dangerous: bool,
    retry_ctx: &mut RetryContext,
) -> Result<ActionResult, ActionError> {
    loop {
        // Capture before state for actions that should change the screen
        if action.should_verify_effect() {
            retry_ctx.capture_before()?;
        }

        // Execute the action
        let result = execute_action(action, confirm_dangerous)?;

        if !result.success {
            if retry_ctx.should_retry() {
                retry_ctx.increment();
                log::warn!(
                    "Action failed, retrying ({}/{}): {:?}",
                    retry_ctx.attempt, retry_ctx.max_retries, action
                );
                std::thread::sleep(Duration::from_millis(retry_ctx.retry_delay_ms as u64));
                continue;
            }
            return Ok(result);
        }

        // For actions that should have visible effect, verify screen changed
        if action.should_verify_effect() {
            std::thread::sleep(Duration::from_millis(200)); // Allow UI to update
            if !retry_ctx.screen_changed()? {
                if retry_ctx.should_retry() {
                    retry_ctx.increment();
                    log::warn!(
                        "Action had no visible effect, retrying ({}/{}): {:?}",
                        retry_ctx.attempt, retry_ctx.max_retries, action
                    );
                    std::thread::sleep(Duration::from_millis(retry_ctx.retry_delay_ms as u64));
                    continue;
                }
                log::warn!("Action completed but no screen change detected");
            }
        }

        return Ok(result);
    }
}

impl Action {
    /// Actions that should produce visible screen changes
    fn should_verify_effect(&self) -> bool {
        matches!(self,
            Action::Click { .. } |
            Action::DoubleClick { .. } |
            Action::Type { .. } |
            Action::Key { .. } |
            Action::Scroll { .. }
        )
    }
}
```

### 4. Update Agent Loop (`src-tauri/src/agent/loop_runner.rs`)

```rust
// In the agent loop, use retry-enabled execution
let retry_ctx = RetryContext::new(
    config.general.max_retries,
    config.general.retry_delay_ms,
);

let result = execute_action_with_retry(&action, confirm_dangerous, &mut retry_ctx).await?;

// Report retry statistics in state
if retry_ctx.attempt > 0 {
    log::info!("Action succeeded after {} retries", retry_ctx.attempt);
    // Optionally emit retry count to frontend
}
```

### 5. Add LLM Retry Guidance (`src-tauri/src/llm/provider.rs`)

Update system prompt to inform the model about retry behavior:
```
Note: Actions are automatically retried up to 3 times if they fail or have no visible effect.
If an action consistently fails, try:
- Using wait action first to ensure element is loaded
- Adjusting coordinates slightly
- Using a different approach (e.g., keyboard navigation instead of clicking)
```

### 6. Update Frontend State Display (`src/main.js`)

Show retry information:
```javascript
// In agent-state handler
if (event.payload.retry_count > 0) {
    actionInfo.textContent += ` (${event.payload.retry_count} retries)`;
}
```

### 7. Add State Fields (`src-tauri/src/agent/state.rs`)

```rust
pub struct AgentStatePayload {
    // ... existing fields
    pub last_retry_count: u32,
    pub total_retries: u32,
}
```

## Acceptance Criteria

- [ ] Failed actions automatically retry up to configured max_retries
- [ ] Retry delay is configurable (default 1000ms)
- [ ] Screen comparison detects when clicks have no effect
- [ ] Retry statistics are tracked and reported
- [ ] UI shows retry count when retries occurred
- [ ] Self-correction can be disabled in settings
- [ ] System prompt informs model about retry behavior
- [ ] Retry count resets between different actions

## Files to Create/Modify

- `src-tauri/src/config/settings.rs` - Add retry configuration
- `src-tauri/src/agent/retry.rs` - New retry module
- `src-tauri/src/agent/mod.rs` - Export retry module
- `src-tauri/src/agent/action.rs` - Add retry wrapper function
- `src-tauri/src/agent/loop_runner.rs` - Use retry-enabled execution
- `src-tauri/src/agent/state.rs` - Add retry statistics
- `src-tauri/src/llm/provider.rs` - Update system prompt
- `src/main.js` - Display retry information

## Integration Points

- **Provides**: Automatic retry capability, reliability improvements
- **Consumes**: Screenshot capture, wait-for-element action
- **Conflicts**: Modifies action execution flow in loop_runner.rs

## Testing Notes

- Test retry on deliberately failing action
- Test retry limit is respected
- Test screen change detection
- Test retry delay timing
- Test retry statistics reporting
- Test with self-correction disabled
- Verify retries don't cause infinite loops
