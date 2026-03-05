---
id: compact-tool-results
name: Make action results more token-efficient
wave: 1
priority: 3
dependencies: []
estimated_hours: 2
tags: [backend, token-optimization, llm]
---

## Objective

Reduce the token footprint of action results that are fed back into the conversation history after each action execution.

## Context

After each action executes, the result is added to the conversation in two places:

1. **`ActionResult::to_tool_result_content()`** (`action.rs:175-186`) - Serializes a full JSON object:
   ```json
   {"status":"success","action":"click","message":"Clicked left at (100, 200)","details":{"type":"click","x":100,"y":200,"button":"left"}}
   ```
   This is ~130+ bytes per action result, and is redundant since the LLM already knows what action it requested.

2. **`conversation.add_tool_result()`** in `loop_runner.rs:448` - Adds the result message to conversation history. Then in `history_to_messages()` (`provider.rs:370-381`), success results become:
   ```
   "Action executed successfully. Clicked left at (100, 200)"
   ```
   And failures become:
   ```
   "Action failed. Element not found"
   ```

Over a 20-message history, these verbose results accumulate significant tokens.

## Implementation

1. **Simplify `to_tool_result_content()`** in `action.rs`:
   - For successful actions: just return `"OK"` or a very short status
   - For failed actions: return `"FAIL: <message>"`
   - Remove the redundant `details` field (the LLM already knows what it asked to do)

2. **Simplify tool result text** in `history_to_messages()` in `provider.rs`:
   - Success: `"OK"` instead of `"Action executed successfully. Clicked left at (100, 200)"`
   - Failure: `"FAIL: <error>"` instead of `"Action failed. <error>"`

3. **Simplify `ActionResult` messages** in `loop_runner.rs`:
   - When calling `conversation.add_tool_result()`, use shorter messages
   - Success actions: just pass `None` for message (or very short confirmation)
   - Failed actions: keep the error message but make it concise

4. **Update tests** in `action.rs` and `provider.rs`

## Acceptance Criteria

- [ ] Successful action results use minimal token-efficient format
- [ ] Failed action results still include the error message
- [ ] All existing tests pass (update assertions)
- [ ] Frontend display is not affected (action results in UI are separate from LLM conversation)
- [ ] Session history export still captures full action details (via ActionEntry, not conversation)

## Files to Create/Modify

- `src-tauri/src/agent/action.rs` - Simplify `to_tool_result_content()`
- `src-tauri/src/llm/provider.rs` - Simplify tool result text in `history_to_messages()`
- `src-tauri/src/agent/loop_runner.rs` - Use shorter messages in `conversation.add_tool_result()`

## Integration Points

- **Provides**: Reduced conversation history token usage
- **Consumes**: ActionResult from action execution
- **Conflicts**: Avoid editing `conversation.rs` (handled by strip-old-screenshots task)
