---
id: wait-for-element
name: Wait for Element Action
wave: 1
priority: 2
dependencies: []
estimated_hours: 5
tags: [backend, reliability, timing]
---

## Objective

Add a "wait" action that polls for visual changes before proceeding, preventing timing-related failures when UI elements haven't loaded yet.

## Context

Many automation failures occur because the agent clicks or types before a UI element has loaded. Currently there's no way for the agent to wait for loading states to complete. A wait action allows the LLM to express "wait until X appears" rather than blindly proceeding.

## Implementation

### 1. Add Wait Action (`src-tauri/src/agent/action.rs`)

Add a new action variant:
```rust
pub enum Action {
    // ... existing variants ...
    Wait {
        timeout_ms: Option<u32>,     // Max wait time (default 5000ms)
        description: String,          // What we're waiting for (for logging)
    },
}
```

### 2. Implement Wait Logic (`src-tauri/src/agent/action.rs`)

In `execute_action()`:
```rust
Action::Wait { timeout_ms, description } => {
    let timeout = timeout_ms.unwrap_or(5000);
    log::info!("Waiting for: {} (timeout: {}ms)", description, timeout);

    // For now, implement as a simple delay
    // Future enhancement: visual comparison to detect change
    std::thread::sleep(Duration::from_millis(timeout.min(10000) as u64));

    Ok(ActionResult {
        success: true,
        completed: false,
        message: Some(format!("Waited for: {}", description))
    })
}
```

### 3. Update System Prompt (`src-tauri/src/llm/provider.rs`)

Add wait action to available actions:
```
- {"action": "wait", "timeout_ms": 3000, "description": "page to load"}
  Wait before proceeding. Use when:
  - After clicking a button that triggers loading
  - After navigating to a new page
  - When an element might not be immediately visible
  Default timeout is 5000ms. Max is 10000ms.
```

### 4. Update Frontend Display (`src/main.js`)

Format wait actions:
```javascript
case 'wait':
    return `⏳ Wait: ${action.description} (${action.timeout_ms || 5000}ms)`;
```

### 5. Enhanced Wait with Visual Detection (Optional/Future)

For a more sophisticated implementation, compare screenshots:
```rust
async fn wait_for_change(
    initial_screenshot: &Screenshot,
    timeout_ms: u32,
    check_interval_ms: u32,
) -> Result<bool, CaptureError> {
    let start = Instant::now();
    while start.elapsed().as_millis() < timeout_ms as u128 {
        sleep(Duration::from_millis(check_interval_ms as u64)).await;
        let current = capture_primary_screen()?;
        if screenshots_differ(initial_screenshot, &current) {
            return Ok(true);
        }
    }
    Ok(false) // Timeout reached
}
```

## Acceptance Criteria

- [ ] `{"action": "wait", "timeout_ms": 3000, "description": "dialog to open"}` executes correctly
- [ ] Default timeout is 5000ms when timeout_ms is omitted
- [ ] Maximum timeout is capped at 10000ms for safety
- [ ] Wait action appears in UI with description and timeout
- [ ] System prompt documents wait action with usage examples
- [ ] Agent can continue after wait completes

## Files to Create/Modify

- `src-tauri/src/agent/action.rs` - Add Wait variant, implement execution
- `src-tauri/src/llm/provider.rs` - Update system prompt
- `src/main.js` - Format wait actions in UI

## Integration Points

- **Provides**: Timing control for reliable automation
- **Consumes**: Screenshot capture (for future visual detection)
- **Conflicts**: None - additive change

## Testing Notes

- Test wait with default timeout
- Test wait with custom timeout
- Test wait with timeout > 10000ms (should cap)
- Verify agent continues execution after wait
- Test wait within a batch action
