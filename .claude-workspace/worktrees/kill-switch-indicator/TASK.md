---
id: kill-switch-indicator
name: Kill Switch Visual Indicator
wave: 1
priority: 3
dependencies: []
estimated_hours: 3
tags: [backend, frontend, safety]
---

## Objective

Add a persistent visual reminder showing the emergency stop keyboard shortcut, with clear visual feedback when the kill switch is active or triggered.

## Context

Users need constant awareness of how to stop the agent immediately:
- Physical keyboard shortcut should always be visible
- Visual distinction when agent is running vs stopped
- Clear feedback when kill switch is triggered
- Reduce anxiety about losing control

Current state:
- Stop button exists but only visible when agent is running
- Global shortcuts plugin is commented out in lib.rs
- No persistent kill switch reminder

## Implementation

### Backend (Rust)

1. **Enable global shortcuts plugin** - `src-tauri/src/lib.rs`
   - Uncomment the global shortcuts plugin (currently commented at line ~109)
   - Register global hotkey for kill switch (e.g., Cmd+Shift+Escape on macOS)

2. **Add kill_switch_triggered to AgentState** - `src-tauri/src/agent/state.rs`
   - Add `kill_switch_triggered: bool` field
   - Method to mark as triggered (separate from normal stop)

3. **Create global shortcut handler** - `src-tauri/src/lib.rs` or new file
   - Register: Cmd+Shift+Escape (macOS), Ctrl+Shift+Escape (Windows/Linux)
   - On trigger: call stop_agent(), set kill_switch_triggered
   - Emit event to frontend for visual feedback

4. **Update agent state emission** - `src-tauri/src/agent/loop_runner.rs`
   - Include kill_switch info in state updates

### Frontend (JavaScript)

5. **Add kill switch indicator to UI** - `src/main.js`
   - Persistent small indicator in corner of modal
   - Shows shortcut: "⌘⇧⎋" (macOS) or "Ctrl+Shift+Esc" (Windows)
   - States:
     - Idle: Gray, subtle
     - Agent running: Pulsing amber/orange (reminder)
     - Triggered: Flash red briefly, then stop

6. **Platform-aware shortcut display** - `src/main.js`
   - Detect platform (Tauri provides this via navigator.platform or config)
   - Show appropriate modifier keys for platform

7. **Add CSS for indicator states** - `index.html`
   - `.kill-switch` base styling
   - `.kill-switch--idle` subtle gray
   - `.kill-switch--armed` pulsing amber (when agent running)
   - `.kill-switch--triggered` red flash animation

8. **Add tooltip on hover** - `src/main.js`
   - Show full shortcut description
   - "Emergency Stop: Press Cmd+Shift+Escape to immediately stop the agent"

## Kill Switch Behavior

| Agent State | Kill Switch Display |
|-------------|---------------------|
| Idle | Gray, subtle, static |
| Running | Amber, pulsing slowly |
| Stopping | Red, solid |
| Triggered | Red flash, then gray |

## Acceptance Criteria

- [ ] Kill switch shortcut always visible in UI
- [ ] Shortcut shows platform-appropriate keys (⌘ vs Ctrl)
- [ ] Visual state changes when agent is running
- [ ] Global hotkey works even when app not focused
- [ ] Triggering hotkey stops agent immediately
- [ ] Visual flash feedback when triggered
- [ ] Tooltip explains the shortcut on hover
- [ ] Works on macOS, Windows, and Linux

## Files to Create/Modify

- `src-tauri/src/lib.rs` - Enable global shortcuts, register kill switch
- `src-tauri/src/agent/state.rs` - Add kill_switch_triggered field
- `src-tauri/src/agent/loop_runner.rs` - Include kill switch state in emissions
- `src/main.js` - Kill switch indicator UI and animations
- `index.html` - CSS for indicator states and animations

## Integration Points

- **Provides**: Emergency stop mechanism with visual feedback
- **Consumes**: None (independent feature)
- **Conflicts**: None - uses existing stop mechanism, adds visual layer

## Platform Notes

- **macOS**: Cmd+Shift+Escape, uses Tauri global shortcuts
- **Windows**: Ctrl+Shift+Escape, may conflict with Task Manager - consider alternative
- **Linux**: Ctrl+Shift+Escape, similar to Windows
- **Alternative shortcuts to consider**: F12, Cmd+., Cmd+Shift+Q
