---
id: instruction-history
name: Instruction History Dropdown
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [frontend, backend, state]
---

## Objective

Add a dropdown/recent list to quickly re-run past instructions without retyping them.

## Context

Currently, the instruction input field only stores the current instruction. Users must retype previous tasks each time. This feature adds persistent history storage and a UI dropdown to recall and re-run past instructions.

## Implementation

### Backend (Rust)

1. **Create history module** `src-tauri/src/history/mod.rs`:
   - Define `InstructionHistory` struct with max 50 entries
   - Store: instruction text, timestamp, success status
   - Persist to `~/.config/pia/history.json`

2. **Add Tauri commands** in `src-tauri/src/lib.rs`:
   - `get_instruction_history()` - returns last 50 instructions
   - `add_to_history(instruction)` - adds new entry
   - `clear_history()` - clears all history
   - `remove_from_history(index)` - removes specific entry

3. **Integrate with agent loop** in `loop_runner.rs`:
   - On successful task completion, save instruction to history
   - Track success/failure status

### Frontend (JavaScript)

4. **Update UI** in `index.html`:
   - Add dropdown button next to instruction input
   - Add history dropdown panel (hidden by default)
   - Style dropdown to match existing dark theme

5. **Add history logic** in `src/main.js`:
   - Load history on app start
   - Show/hide dropdown on button click
   - Click history item to populate input
   - Double-click to run immediately
   - Add clear history button

## Acceptance Criteria

- [ ] History persists across app restarts
- [ ] Dropdown shows last 50 instructions with timestamps
- [ ] Clicking an item populates the input field
- [ ] Double-clicking runs the instruction immediately
- [ ] Clear history button works
- [ ] History entries show success/failure indicator
- [ ] No performance impact on app startup

## Files to Create/Modify

- `src-tauri/src/history/mod.rs` - NEW: History storage module
- `src-tauri/src/lib.rs` - Add history commands and module import
- `src-tauri/src/agent/loop_runner.rs` - Add history tracking on completion
- `index.html` - Add dropdown UI elements and styles
- `src/main.js` - Add history management functions

## Integration Points

- **Provides**: Instruction history for quick recall
- **Consumes**: Agent completion events
- **Conflicts**: None - isolated feature
