---
id: fix-conversation-model
name: Extend conversation model to support native tool_use metadata
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [backend, agent, conversation]
---

## Objective

Extend the `Message` enum in `conversation.rs` to carry native tool_use metadata (tool_use_id, tool_name, tool_input) so the Anthropic provider can format proper tool_use/tool_result content blocks.

## Context

Currently, the `Message` enum has 3 variants: `User`, `Assistant` (stores raw text), and `ToolResult` (success/message/error). When Anthropic returns a `tool_use` response, it's stored via `conversation.add_assistant_message(&response_str)` as a plain JSON string. The tool result is stored via `conversation.add_tool_result(true, message, None)` with no tool_use_id.

The Anthropic API requires:
- **Assistant messages** to contain `tool_use` content blocks: `{ type: "tool_use", id: "toolu_xxx", name: "click", input: {...} }`
- **User messages** to contain `tool_result` content blocks: `{ type: "tool_result", tool_use_id: "toolu_xxx", content: "..." }`

Without proper tool_use_id threading, multi-turn tool calling breaks after the first action.

## Implementation

1. **`src-tauri/src/agent/conversation.rs`** — Add a new `AssistantToolUse` variant to the `Message` enum:
   ```rust
   /// Assistant response using native tool_use (Anthropic)
   AssistantToolUse {
       tool_use_id: String,
       tool_name: String,
       tool_input: serde_json::Value,
       /// Optional text content before tool_use (thinking/reasoning)
       #[serde(skip_serializing_if = "Option::is_none")]
       text: Option<String>,
   },
   ```

2. **`src-tauri/src/agent/conversation.rs`** — Modify `ToolResult` variant to include `tool_use_id`:
   ```rust
   ToolResult {
       success: bool,
       #[serde(skip_serializing_if = "Option::is_none")]
       tool_use_id: Option<String>,
       #[serde(skip_serializing_if = "Option::is_none")]
       message: Option<String>,
       #[serde(skip_serializing_if = "Option::is_none")]
       error: Option<String>,
   },
   ```

3. **`src-tauri/src/agent/conversation.rs`** — Add helper methods:
   ```rust
   /// Adds an assistant tool_use message (for native tool_use providers like Anthropic)
   pub fn add_assistant_tool_use(
       &mut self,
       tool_use_id: String,
       tool_name: String,
       tool_input: serde_json::Value,
       text: Option<String>,
   ) { ... }

   /// Adds a tool result with tool_use_id for proper pairing
   pub fn add_tool_result_with_id(
       &mut self,
       tool_use_id: String,
       success: bool,
       message: Option<String>,
       error: Option<String>,
   ) { ... }
   ```

4. **`src-tauri/src/agent/conversation.rs`** — Update existing `add_tool_result()` to set `tool_use_id: None` for backward compatibility with non-Anthropic providers.

5. **Update existing tests** and add new tests:
   - Test `add_assistant_tool_use()` stores correct fields
   - Test `add_tool_result_with_id()` stores tool_use_id
   - Test serialization/deserialization of new variants
   - Test backward compatibility: `add_tool_result()` still works without ID

## Acceptance Criteria

- [ ] `Message::AssistantToolUse` variant exists with tool_use_id, tool_name, tool_input, optional text
- [ ] `Message::ToolResult` has optional tool_use_id field
- [ ] `add_assistant_tool_use()` method works correctly
- [ ] `add_tool_result_with_id()` method works correctly
- [ ] Existing `add_tool_result()` still works (backward compatible)
- [ ] `last_assistant_message()` works with both `Assistant` and `AssistantToolUse` variants
- [ ] All existing tests pass
- [ ] New tests cover serialization roundtrip of new variants

## Files to Create/Modify

- `src-tauri/src/agent/conversation.rs` — Add `AssistantToolUse` variant, modify `ToolResult`, add helper methods

## Integration Points

- **Provides**: Extended conversation model that can carry tool_use metadata for Anthropic
- **Consumes**: Nothing new (self-contained data model change)
- **Conflicts**: Do NOT modify `loop_runner.rs` or `provider.rs` — those changes are in `fix-anthropic-tool-flow`
