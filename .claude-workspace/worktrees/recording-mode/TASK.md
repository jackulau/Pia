---
id: recording-mode
name: Recording Mode - Watch-only mode that captures what the agent would do
wave: 1
priority: 3
dependencies: []
estimated_hours: 5
tags: [backend, frontend, safety]
---

## Objective

Implement a "Recording Mode" that runs the agent loop but captures planned actions without executing them, allowing users to preview what the agent would do before committing to actual execution.

## Context

Users may want to see the agent's plan before it takes control of their computer. Recording mode provides a safe preview mechanism - the agent analyzes screenshots and determines actions, but doesn't execute them. Users can review the proposed actions and decide whether to proceed.

## Implementation

### Backend (Rust)

1. **Add Recording Mode to State** (`src-tauri/src/agent/state.rs`)
   - Add `execution_mode: ExecutionMode` enum (Normal, Recording)
   - Add `recorded_actions: Vec<RecordedAction>` for storing planned actions
   - `RecordedAction` struct: action, screenshot_before, llm_reasoning, timestamp

2. **Modify Loop Runner** (`src-tauri/src/agent/loop_runner.rs`)
   - Check execution_mode before action execution
   - In Recording mode: capture action but skip execute_action()
   - Still emit state updates so UI shows planned actions
   - Continue loop to capture full action sequence
   - Add iteration limit for recording (prevent infinite recording)

3. **Add Tauri Commands** (`src-tauri/src/lib.rs`)
   - `start_agent_recording(instruction: String)` - Start in recording mode
   - `get_recorded_actions()` - Return list of recorded actions
   - `execute_recorded_actions()` - Run the captured actions (optional)
   - `clear_recorded_actions()` - Discard recording

### Frontend (JavaScript)

4. **Recording UI** (`index.html`)
   - Add "Record" button alongside Submit button
   - Recording indicator (pulsing dot) when in recording mode
   - Recorded actions display panel showing captured actions
   - "Execute All" button to run recorded sequence
   - "Clear" button to discard recording

5. **Recording Logic** (`src/main.js`)
   - `startRecording()` - Invoke start_agent_recording
   - Display recorded actions in list format
   - Handle recording completion state
   - `executeRecordedActions()` - Run captured sequence

## Acceptance Criteria

- [ ] "Record" button starts agent in recording mode
- [ ] Agent analyzes screenshots and plans actions without executing
- [ ] Planned actions are captured with LLM reasoning
- [ ] UI clearly indicates recording mode (visual indicator)
- [ ] Recorded actions display in a reviewable list
- [ ] Users can execute recorded actions after review
- [ ] Users can clear/discard recorded actions
- [ ] Recording stops when agent completes or reaches limit
- [ ] Normal execution still works unchanged
- [ ] Recording mode respects same iteration limits as normal mode

## Files to Create/Modify

- `src-tauri/src/agent/state.rs` - Add ExecutionMode enum and recorded_actions
- `src-tauri/src/agent/loop_runner.rs` - Conditional execution based on mode
- `src-tauri/src/lib.rs` - Add recording commands
- `index.html` - Add Record button, indicator, and action review panel
- `src/main.js` - Add recording handlers and action display
- `src/styles/main.css` - Recording mode visual styles

## Integration Points

- **Provides**: Safe preview/dry-run capability
- **Consumes**: Existing AgentLoop, action parsing
- **Conflicts**: Modifies loop_runner.rs and state.rs (coordinate carefully)

## Technical Notes

- Recording mode should still capture screenshots at each step
- Consider storing screenshot thumbnails for recorded actions display
- Execution of recorded actions could be step-by-step or all-at-once
- May want to add pause/step-through for recorded execution
- Action list format in UI:
  ```
  1. Click at (150, 200) - "Opening Chrome browser"
  2. Wait 500ms
  3. Type "weather forecast" - "Entering search query"
  4. Key Enter - "Submitting search"
  ```
- Consider adding timestamps to recorded actions
