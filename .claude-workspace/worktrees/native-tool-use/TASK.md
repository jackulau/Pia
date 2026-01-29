---
id: native-tool-use
name: Implement Native Claude Tool Use Protocol
wave: 1
priority: 1
dependencies: []
estimated_hours: 6
tags: [backend, llm, critical]
---

## Objective

Implement Anthropic's native tool_use protocol in the Anthropic provider instead of relying on JSON in system prompts.

## Context

Currently, the system embeds action definitions in the system prompt and hopes the LLM returns valid JSON. This is fragile and doesn't leverage Claude's structured tool calling capabilities. Native tool_use provides:
- Structured tool definitions with JSON schemas
- Guaranteed valid tool_use responses
- Better error handling and validation
- Tool result feedback loop

## Implementation

1. Modify `/src-tauri/src/llm/provider.rs`:
   - Add `Tool` struct with name, description, input_schema fields
   - Add `build_tools()` function returning Vec<Tool> for all actions
   - Update `LlmProvider` trait to include tools in request

2. Modify `/src-tauri/src/llm/anthropic.rs`:
   - Update `AnthropicRequest` to include `tools` field
   - Parse `tool_use` content blocks from response
   - Extract tool name and input from structured response
   - Handle `tool_result` messages for multi-turn tool use

3. Update action parsing in `/src-tauri/src/agent/action.rs`:
   - Add `from_tool_use()` method to parse tool_use blocks
   - Keep existing JSON parsing as fallback for other providers

## Acceptance Criteria

- [ ] Tool definitions are sent via `tools` API parameter
- [ ] Response parsing handles `tool_use` content blocks
- [ ] Action extraction works from structured tool responses
- [ ] Fallback to JSON parsing for non-Anthropic providers
- [ ] No regressions in existing functionality
- [ ] Token counting includes tool definitions

## Files to Create/Modify

- `src-tauri/src/llm/provider.rs` - Add Tool struct and build_tools()
- `src-tauri/src/llm/anthropic.rs` - Implement native tool_use protocol
- `src-tauri/src/agent/action.rs` - Add from_tool_use() parsing

## Integration Points

- **Provides**: Native tool calling for Anthropic provider
- **Consumes**: Existing action enum definitions
- **Conflicts**: Avoid modifying loop_runner.rs (handled by conversation-context task)
