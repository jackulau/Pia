---
id: async-sleep-fix
name: Replace Blocking Sleeps with Async
wave: 1
priority: 1
dependencies: []
estimated_hours: 2
tags: [backend, critical, performance]
---

## Objective

Replace synchronous `std::thread::sleep` calls with async `tokio::time::sleep` to prevent blocking the async runtime.

## Context

The codebase has two critical blocking operations that block the entire async runtime thread:
1. `src-tauri/src/agent/action.rs` line 218 - 50ms sleep in scroll action
2. `src-tauri/src/input/mouse.rs` line 101 - 50ms sleep in click_at()

These blocking calls prevent other async tasks from executing and degrade overall responsiveness.

## Implementation

1. In `src-tauri/src/agent/action.rs`:
   - Change `std::thread::sleep(std::time::Duration::from_millis(50))` to `tokio::time::sleep(Duration::from_millis(50)).await`
   - Ensure the function is async or wrapped appropriately

2. In `src-tauri/src/input/mouse.rs`:
   - Change the synchronous sleep to async
   - Update function signature to be async if needed
   - Update all callers to await the result

3. Review all usages of `std::thread::sleep` in the codebase and replace with async equivalents

## Acceptance Criteria

- [ ] No `std::thread::sleep` calls remain in async code paths
- [ ] All sleep operations use `tokio::time::sleep`
- [ ] Code compiles without errors
- [ ] Agent loop runs without blocking
- [ ] Manual testing confirms no regressions in click/scroll actions

## Files to Create/Modify

- `src-tauri/src/agent/action.rs` - Replace blocking sleep in scroll handler
- `src-tauri/src/input/mouse.rs` - Replace blocking sleep in click_at()
- `src-tauri/src/input/mod.rs` - Update exports if needed

## Integration Points

- **Provides**: Non-blocking input operations
- **Consumes**: tokio runtime
- **Conflicts**: None - isolated changes to input handling
