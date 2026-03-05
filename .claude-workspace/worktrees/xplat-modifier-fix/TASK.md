---
id: xplat-modifier-fix
name: Fix hardcoded cmd modifier in action reversal
wave: 1
priority: 1
dependencies: []
estimated_hours: 2
tags: [backend, bug, cross-platform, P0]
---

## Objective

Fix the hardcoded `"cmd"` modifier in `Action::create_reverse()` so undo actions use the correct platform modifier (`"ctrl"` on Windows/Linux, `"cmd"` on macOS).

## Context

In `src-tauri/src/agent/action.rs`, the `create_reverse()` method generates undo actions (Cmd+Z for Type, Cmd+Z for Key). It hardcodes `"cmd"` as the modifier in 3 locations (~lines 1079-1093). On Windows and Linux, this should be `"ctrl"`. The `is_reversible()` method already checks for both `"cmd"` and `"ctrl"`, so only `create_reverse()` is broken.

This is a P0 bug — undo/recovery is completely broken on non-macOS platforms.

## Implementation Steps

1. **Add a platform-aware modifier helper function** in `action.rs` (or a shared utility):
   ```rust
   fn platform_cmd_modifier() -> String {
       if cfg!(target_os = "macos") {
           "cmd".to_string()
       } else {
           "ctrl".to_string()
       }
   }
   ```

2. **Replace all 3 hardcoded `"cmd"` strings** in `create_reverse()` with calls to this helper.

3. **Add unit tests** for `create_reverse()` that verify:
   - On the current platform, the correct modifier is used
   - The generated reverse action is a valid `Action::Key` with `key: "z"`

4. **Audit the rest of action.rs** for any other hardcoded `"cmd"` strings that should be platform-aware. Check `execute_action_with_delay` and `create_reverse` thoroughly.

## Acceptance Criteria

- [ ] `create_reverse()` returns `"ctrl"` modifier on Windows/Linux, `"cmd"` on macOS
- [ ] All existing tests pass (`cargo test` in `src-tauri/`)
- [ ] New unit tests cover the platform modifier behavior
- [ ] No other hardcoded `"cmd"` strings remain that should be platform-aware

## Files to Create/Modify

- **Modify:** `src-tauri/src/agent/action.rs` — add helper function, fix `create_reverse()`, add tests

## Integration Points

- `create_reverse()` is called from `agent/history.rs` via `ActionHistory::undo_last()`
- The `undo_last_action` Tauri command in `lib.rs` triggers this chain
- The generated `Action::Key` is executed via `execute_action_with_delay` which calls `KeyboardController`
