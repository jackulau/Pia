---
id: fix-tool-parity
name: Fix Tool Definition Parity and UTF-8 Bug
wave: 1
priority: 1
dependencies: []
estimated_hours: 2
tags: [backend, agent, bugfix]
---

## Objective

Add all 6 missing action types to Anthropic's native tool_use definitions and fix the UTF-8 truncation panic.

## Context

Currently `build_tools()` in `src-tauri/src/llm/provider.rs` only defines 8 tools for Anthropic's native tool_use API (click, double_click, move, type, key, scroll, complete, error), but the JSON system prompt for non-tool providers documents 14 action types. This means Anthropic-based agents cannot use drag, triple_click, right_click, wait, batch, or wait_for_element — significantly limiting their capability for case-specific tasks.

Additionally, `truncate_string()` in `src-tauri/src/agent/action.rs` (around line 863) slices at byte boundaries, which will panic on multi-byte UTF-8 characters.

## Implementation

1. **Expand `build_tools()`** in `src-tauri/src/llm/provider.rs` (starts around line 80):
   - Add `drag` tool: `start_x`, `start_y`, `end_x`, `end_y`, `button` (optional), `duration_ms` (optional, max 5000)
   - Add `triple_click` tool: `x`, `y`
   - Add `right_click` tool: `x`, `y`
   - Add `wait` tool: `duration_ms` (max 5000)
   - Add `batch` tool: `actions` (array, max 10 items)
   - Add `wait_for_element` tool: `description`, `timeout_ms` (max 10000)
   - Follow the exact same JSON schema pattern used by existing tool definitions

2. **Verify `from_tool_use()`** in `src-tauri/src/agent/action.rs` (starts around line 224):
   - Confirm all 14 action types are handled in the tool_use parsing path
   - Add any missing match arms if needed

3. **Fix `truncate_string()`** in `src-tauri/src/agent/action.rs` (around line 863):
   - Replace byte slicing with `.chars().take(n).collect::<String>()` or use `str::char_indices()` to find the correct boundary

4. **Update `build_system_prompt_for_tools()`** in `provider.rs` (around line 388):
   - Add brief guidance about the new tool types in the system prompt for tool-use providers
   - Mention drag for moving elements, triple_click for line selection, right_click for context menus, wait for timing, batch for sequences

5. **Add tests** for the new tool definitions:
   - Verify all 14 tools are present in `build_tools()` output
   - Test `from_tool_use()` parsing for all 14 action types
   - Test `truncate_string()` with multi-byte UTF-8 strings

## Acceptance Criteria

- [ ] `build_tools()` returns definitions for all 14 action types
- [ ] `from_tool_use()` correctly parses all 14 action types from tool_use format
- [ ] `truncate_string()` handles multi-byte UTF-8 without panicking
- [ ] Existing tests still pass (`cargo test` in src-tauri/)
- [ ] New tests cover the added tool definitions and UTF-8 fix

## Files to Create/Modify

- `src-tauri/src/llm/provider.rs` - Add 6 tool definitions to `build_tools()`, update system prompt
- `src-tauri/src/agent/action.rs` - Verify `from_tool_use()` handles all types, fix `truncate_string()`

## Integration Points

- **Provides**: Full action type parity for Anthropic tool_use providers
- **Consumes**: None (core fix)
- **Conflicts**: Avoid editing `loop_runner.rs` or `conversation.rs` (other tasks may touch those)
