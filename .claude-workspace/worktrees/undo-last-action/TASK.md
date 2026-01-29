---
id: undo-last-action
name: Undo Last Action
wave: 2
priority: 4
dependencies: [preview-mode]
estimated_hours: 6
tags: [backend, frontend, safety, complex]
---

## Objective

Implement an undo mechanism that can reverse the last agent action where possible, with clear feedback about what can and cannot be undone.

## Context

Actions currently execute without history tracking. Users need:
- Ability to undo accidental actions
- Understanding of which actions are reversible
- Confidence that mistakes can be corrected
- History of recent actions for reference

**Dependency**: Relies on preview-mode for action display infrastructure.

## Implementation

### Backend (Rust)

1. **Create ActionHistory** - `src-tauri/src/agent/history.rs` (new file)
   ```rust
   pub struct ActionRecord {
       action: Action,
       timestamp: DateTime<Utc>,
       success: bool,
       reversible: bool,
       reverse_action: Option<Action>,
   }
   
   pub struct ActionHistory {
       records: VecDeque<ActionRecord>,
       max_size: usize,  // e.g., 50 actions
   }
   
   impl ActionHistory {
       pub fn push(&mut self, record: ActionRecord);
       pub fn pop_last(&mut self) -> Option<ActionRecord>;
       pub fn get_last(&self) -> Option<&ActionRecord>;
       pub fn can_undo(&self) -> bool;
   }
   ```

2. **Define reversibility rules** - `src-tauri/src/agent/action.rs`
   - Add `is_reversible(&self) -> bool` method to Action
   - Add `create_reverse(&self) -> Option<Action>` method
   
   | Action | Reversible | Reverse Action |
   |--------|------------|----------------|
   | Type | Yes | Select all + Delete (for that text) |
   | Key (typing) | Partial | Backspace |
   | Key (Cmd+Z) | No | Already an undo |
   | Click | No | Can't unclick |
   | Move | Maybe | Move back to original position (stored) |
   | Scroll | Yes | Scroll opposite direction |
   | Double Click | No | Can't reverse |

3. **Track actions in loop_runner** - `src-tauri/src/agent/loop_runner.rs`
   - Add ActionHistory to AgentLoop
   - Before execute: create ActionRecord with potential reverse
   - After execute: mark success, store in history
   - Emit history update to frontend

4. **Add undo command** - `src-tauri/src/lib.rs`
   - `undo_last_action()` command
   - Pop last action from history
   - Execute reverse action if available
   - Return result to frontend

5. **Update AgentState** - `src-tauri/src/agent/state.rs`
   - Add `can_undo: bool` field
   - Add `last_undoable_action: Option<String>` for UI display

### Frontend (JavaScript)

6. **Add Undo button to UI** - `src/main.js`
   - Button near stop button (or in action area)
   - Disabled when nothing to undo
   - Shows what will be undone on hover
   - Keyboard shortcut: Cmd+Z (when agent stopped)

7. **Update action display** - `src/main.js`
   - Show reversibility indicator for each action
   - Icon: ↩️ if reversible, ⚠️ if not
   - Tooltip explaining reversibility

8. **Add action history panel** (optional) - `src/main.js`
   - Collapsible list of recent actions
   - Click to see details
   - "Undo" button per reversible action (maybe v2)

9. **Add CSS for undo UI** - `index.html`
   - Undo button styling
   - Disabled state styling
   - Reversibility indicator styling

## Undo Behavior Details

### Type Action Undo
```javascript
// Original: type "Hello World"
// Undo: Select text range, delete
// Challenge: Need to track cursor position before/after
```

### Key Action Undo
```javascript
// Original: key "Enter"
// Undo: Backspace (partial - removes newline)
// Some keys can't be undone (Escape, Tab sometimes)
```

### Scroll Undo
```javascript
// Original: scroll down 3 units at (500, 300)
// Undo: scroll up 3 units at (500, 300)
```

## Acceptance Criteria

- [ ] Action history maintained (last 50 actions)
- [ ] Each action marked as reversible or not
- [ ] Undo button visible when undo is possible
- [ ] Undo button disabled when nothing to undo
- [ ] Successful undo shows feedback message
- [ ] Failed undo shows appropriate error
- [ ] Cmd+Z keyboard shortcut works (when agent stopped)
- [ ] Scroll actions properly reverse
- [ ] Type actions attempt reversal (best effort)
- [ ] Action history clears on new agent session

## Files to Create/Modify

- `src-tauri/src/agent/history.rs` - NEW: ActionHistory struct
- `src-tauri/src/agent/mod.rs` - Export history module
- `src-tauri/src/agent/action.rs` - Add reversibility logic
- `src-tauri/src/agent/loop_runner.rs` - Track action history
- `src-tauri/src/agent/state.rs` - Add undo state fields
- `src-tauri/src/lib.rs` - Add undo_last_action command
- `src/main.js` - Undo button and history display
- `index.html` - CSS for undo UI elements

## Integration Points

- **Provides**: Action undo capability for user safety
- **Consumes**: Action execution flow from action.rs
- **Depends on**: preview-mode for action display infrastructure
- **Conflicts**: Avoid modifying action parsing (handled by preview-mode if relevant)

## Complexity Notes

This is the most complex task in the Safety & Control set because:
1. Reversibility is context-dependent (can't always undo)
2. Text undo requires cursor position tracking
3. Some "undos" may cause unintended effects
4. Need to handle partial reversals gracefully

Consider starting with scroll and simple key reversals, then expanding to type.
