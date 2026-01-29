---
id: conversation-context
name: Implement Conversation History and Context Management
wave: 1
priority: 2
dependencies: []
estimated_hours: 5
tags: [backend, state, critical]
---

## Objective

Add conversation history management so the LLM has context of previous actions and can make better decisions.

## Context

Currently, each iteration starts fresh - the LLM only receives the current screenshot and original instruction. This means:
- No memory of what actions were already tried
- Cannot learn from failed attempts
- Repeats the same action if it didn't work
- No way to track multi-step sequences

## Implementation

1. Create new file `/src-tauri/src/agent/conversation.rs`:
   - Define `Message` enum (User, Assistant, ToolUse, ToolResult)
   - Create `ConversationHistory` struct to track messages
   - Implement methods: add_message(), get_messages(), clear()
   - Add serialization for API message format

2. Modify `/src-tauri/src/agent/state.rs`:
   - Add `conversation: ConversationHistory` field to AgentState
   - Update state initialization

3. Modify `/src-tauri/src/agent/loop_runner.rs`:
   - Initialize conversation at start of task
   - Add user message with instruction + screenshot
   - Add assistant message after LLM response
   - Add tool_result after action execution
   - Pass full conversation history to LLM provider

4. Update `/src-tauri/src/llm/provider.rs`:
   - Modify trait to accept conversation history
   - Update `send_with_image()` signature

## Acceptance Criteria

- [ ] Conversation history is maintained across iterations
- [ ] Previous actions are visible to the LLM
- [ ] Tool results are included in history
- [ ] History is cleared when new task starts
- [ ] Memory usage is bounded (max history length)
- [ ] History is serializable for debugging

## Files to Create/Modify

- `src-tauri/src/agent/conversation.rs` - NEW: Conversation management
- `src-tauri/src/agent/mod.rs` - Export conversation module
- `src-tauri/src/agent/state.rs` - Add conversation to state
- `src-tauri/src/agent/loop_runner.rs` - Use conversation history
- `src-tauri/src/llm/provider.rs` - Accept history in trait

## Integration Points

- **Provides**: Conversation context for all providers
- **Consumes**: Existing state management
- **Conflicts**: Avoid modifying action.rs (handled by other tasks)
