---
id: thinking-display
name: Thinking/Reasoning Display
wave: 1
priority: 2
dependencies: []
estimated_hours: 4
tags: [frontend, backend, ui]
---

## Objective

Show a brief summary of why the agent chose an action, not just what it did - displaying the agent's reasoning/thinking process.

## Context

Currently, the UI only shows "Last Action: Click left at (500, 300)" but doesn't explain why the agent chose that action. Users don't understand the agent's decision-making process, making it harder to trust or debug the agent's behavior.

The LLM response already includes reasoning text before the JSON action. We need to extract and display this thinking/reasoning.

## Implementation

### Backend Changes (src-tauri/src/)

1. **Modify `agent/state.rs`**:
   - Add `last_reasoning: Option<String>` field to `AgentState` struct
   - This will store the thinking/reasoning text from the LLM response

2. **Modify `agent/action.rs`**:
   - Update `parse_action()` to return both the action AND the reasoning text
   - Extract text before the JSON block as reasoning
   - Return type: `(Action, Option<String>)` or a struct with both fields

3. **Modify `agent/loop_runner.rs`**:
   - Capture the reasoning text from `parse_action()`
   - Store it in state via `set_last_reasoning()`
   - Emit with state updates

### Frontend Changes (src/, index.html)

4. **Update `index.html`**:
   - Add a thinking display section:
   ```html
   <div class="thinking-display">
     <div class="thinking-label">Agent Thinking</div>
     <div class="thinking-content" id="thinking-content">Analyzing screen...</div>
   </div>
   ```

5. **Update `src/styles/modal.css`**:
   - Style the thinking display (smaller text, muted color, max-height with scroll)
   - Keep it compact but readable
   - Optional: collapsible/expandable

6. **Update `src/main.js`**:
   - In `updateAgentState()`, update the thinking content
   - Truncate long reasoning (show first ~100 chars with "...")
   - Handle missing reasoning gracefully

## Acceptance Criteria

- [ ] Reasoning text extracted from LLM response (text before JSON)
- [ ] Reasoning displayed in UI with each state update
- [ ] Long reasoning is truncated with ellipsis (configurable limit)
- [ ] Reasoning updates in real-time with each iteration
- [ ] Graceful handling when no reasoning is available
- [ ] Styling is consistent with existing UI (muted, compact)

## Files to Create/Modify

- `src-tauri/src/agent/state.rs` - Add reasoning field to state
- `src-tauri/src/agent/action.rs` - Extract reasoning from LLM response
- `src-tauri/src/agent/loop_runner.rs` - Pass reasoning to state
- `index.html` - Add thinking display HTML
- `src/styles/modal.css` - Style thinking section
- `src/main.js` - Update thinking content on state changes

## Integration Points

- **Provides**: Transparency into agent decision-making
- **Consumes**: LLM response text (already available)
- **Conflicts**: None - additive changes only

## Technical Notes

- LLM responses typically look like:
  ```
  I can see the login page. I'll click on the username field to start entering credentials.
  
  ```json
  {"action": "click", "x": 500, "y": 300}
  ```
  ```
- The text before the JSON block is the reasoning to extract
- Consider extracting the last paragraph before JSON for conciseness
- Current `parse_action()` discards this text - we need to capture it
