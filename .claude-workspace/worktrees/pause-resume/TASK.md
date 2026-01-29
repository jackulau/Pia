---
id: pause-resume
name: Pause/Resume Agent Functionality
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [frontend, backend, state]
---

## Objective

Add user-initiated pause/resume capability so users can intervene during agent execution without losing context.

## Context

Currently there's only a Stop button that completely terminates the agent loop. The `AgentStatus::Paused` enum variant exists but is only used internally for confirmation dialogs. This feature exposes pause/resume to users, allowing them to:
- Pause to review what the agent is doing
- Make manual adjustments before resuming
- Continue without losing iteration state and context

## Implementation

### Backend (Rust)

1. **Enhance state management** in `src-tauri/src/agent/state.rs`:
   - Add `request_pause()` method (similar to `request_stop()`)
   - Add `should_pause: Arc<AtomicBool>` flag
   - Add `resume()` method to clear pause flag
   - Modify `should_stop()` check to also handle pause

2. **Add Tauri commands** in `src-tauri/src/lib.rs`:
   - `pause_agent()` - sets pause flag
   - `resume_agent()` - clears pause flag and continues

3. **Update agent loop** in `src-tauri/src/agent/loop_runner.rs`:
   - Check for pause signal at start of each iteration
   - When paused, emit state update and wait in a loop
   - Resume when flag is cleared
   - Preserve all state (iteration count, instruction, etc.)

### Frontend (JavaScript)

4. **Update UI** in `index.html`:
   - Replace single Stop button with Pause/Resume toggle
   - Add visual indicator when paused (pulsing status)
   - Style buttons to match existing theme

5. **Add pause/resume logic** in `src/main.js`:
   - Track paused state
   - Toggle between Pause and Resume buttons
   - Update status display for paused state
   - Enable instruction editing while paused (optional)

## Acceptance Criteria

- [ ] Pause button visible while agent is running
- [ ] Clicking Pause transitions to Paused state
- [ ] Resume button visible while paused
- [ ] Clicking Resume continues from exact iteration
- [ ] No state lost during pause (iteration, tokens, etc.)
- [ ] UI shows clear paused indicator
- [ ] Stop button remains available while paused
- [ ] Agent can be stopped while paused

## Files to Create/Modify

- `src-tauri/src/agent/state.rs` - Add pause/resume state management
- `src-tauri/src/lib.rs` - Add pause_agent, resume_agent commands
- `src-tauri/src/agent/loop_runner.rs` - Add pause check in loop
- `index.html` - Update button layout and styles
- `src/main.js` - Add pause/resume handlers and state

## Integration Points

- **Provides**: Pause/Resume capability for agent execution
- **Consumes**: Agent state events
- **Conflicts**: Modifies stop button area - coordinate with other UI changes
