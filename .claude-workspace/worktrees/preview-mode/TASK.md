---
id: preview-mode
name: Preview Mode - Dry Run Actions
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [backend, frontend, safety]
---

## Objective

Add a "Preview Mode" toggle that shows planned actions without executing them, allowing users to see what the agent would do before committing to real actions.

## Context

Users need a way to preview agent behavior before letting it take control. This is especially important for:
- Testing new instructions safely
- Understanding agent decision-making
- Debugging unexpected behaviors
- Building trust with the agent

## Implementation

### Backend (Rust)

1. **Add preview_mode to config** - `src-tauri/src/config/settings.rs`
   - Add `preview_mode: bool` field to `GeneralConfig` struct
   - Default to `false`

2. **Modify AgentState** - `src-tauri/src/agent/state.rs`
   - Add `preview_mode: bool` field to `AgentState`
   - Add getter/setter methods

3. **Update loop_runner** - `src-tauri/src/agent/loop_runner.rs`
   - Check `preview_mode` before calling `execute_action()`
   - If preview mode: skip execution, still emit action to frontend
   - Add "[PREVIEW]" prefix to last_action when in preview mode

4. **Add Tauri command** - `src-tauri/src/lib.rs`
   - `set_preview_mode(enabled: bool)` command
   - Update agent state when called

### Frontend (JavaScript)

5. **Add Preview toggle to UI** - `src/main.js`
   - Add toggle switch in settings panel or main UI
   - Style: clear "Preview Mode" label with ON/OFF indicator
   - When enabled, show visual indicator (e.g., dashed border, "PREVIEW" badge)

6. **Update action display** - `src/main.js`
   - When preview mode active, style actions differently
   - Add "Would execute:" prefix to action text
   - Use orange/yellow color scheme for preview actions

7. **Add CSS styles** - `index.html` (inline styles)
   - `.preview-mode` class for main modal
   - `.preview-action` class for action display
   - Toggle switch styling

## Acceptance Criteria

- [ ] Preview mode toggle visible in UI
- [ ] When enabled, agent runs full loop but skips action execution
- [ ] Actions displayed with clear "[PREVIEW]" or "Would execute:" indicator
- [ ] Visual distinction between preview and live mode (border color, badge)
- [ ] Preview mode state persists in config
- [ ] Agent status shows "Preview" when in preview mode
- [ ] Can toggle preview mode while agent is stopped
- [ ] Screenshots still captured in preview mode (to show what agent sees)

## Files to Create/Modify

- `src-tauri/src/config/settings.rs` - Add preview_mode config field
- `src-tauri/src/agent/state.rs` - Add preview_mode state
- `src-tauri/src/agent/loop_runner.rs` - Skip execution in preview mode
- `src-tauri/src/lib.rs` - Add set_preview_mode command
- `src/main.js` - Preview toggle UI and action styling
- `index.html` - Add CSS for preview mode styling

## Integration Points

- **Provides**: Preview mode infrastructure for safe testing
- **Consumes**: None (independent feature)
- **Conflicts**: None - additive change to existing action flow
