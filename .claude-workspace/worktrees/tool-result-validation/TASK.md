---
id: tool-result-validation
name: Implement Tool Result Validation and Feedback
wave: 2
priority: 2
dependencies: [native-tool-use]
estimated_hours: 4
tags: [backend, validation]
---

## Objective

Add tool result validation and feedback mechanism so the LLM receives confirmation of action success/failure.

## Context

Currently, actions execute but results are not fed back to the LLM. The agent simply takes a new screenshot and hopes the LLM notices changes. With native tool_use, we can send `tool_result` messages to provide explicit feedback about:
- Whether the action succeeded or failed
- Error messages if the action failed
- Any relevant output from the action

## Implementation

1. Modify `/src-tauri/src/agent/action.rs`:
   - Enhance `ActionResult` to include structured result data
   - Add serialization for tool_result format
   - Include action-specific result details (e.g., click position confirmed)

2. Modify `/src-tauri/src/llm/anthropic.rs`:
   - Add `tool_result` message type support
   - Send tool_result after each tool_use
   - Handle multi-turn conversations with tool results

3. Update `/src-tauri/src/llm/provider.rs`:
   - Add `send_tool_result()` method to trait
   - Define `ToolResult` struct for standardized results

## Acceptance Criteria

- [ ] ActionResult contains structured result data
- [ ] Tool results are sent back to Anthropic API
- [ ] Multi-turn tool conversations work correctly
- [ ] Error results include helpful error messages
- [ ] Success results confirm action execution

## Files to Create/Modify

- `src-tauri/src/agent/action.rs` - Enhanced ActionResult
- `src-tauri/src/llm/provider.rs` - ToolResult struct and trait method
- `src-tauri/src/llm/anthropic.rs` - tool_result message handling

## Integration Points

- **Provides**: Tool result feedback for conversation
- **Consumes**: native-tool-use implementation
- **Conflicts**: None - builds on native-tool-use
