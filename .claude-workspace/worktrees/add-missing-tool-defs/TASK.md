---
id: add-missing-tool-defs
name: Add missing native tool definitions for Anthropic
wave: 1
priority: 1
dependencies: []
estimated_hours: 3
tags: [backend, llm, tools]
---

## Objective

Add the 6 missing tool definitions to `build_tools()` so Anthropic can use all 14 action types via native tool_use.

## Context

The `build_tools()` function in `provider.rs` only defines 8 tools (click, double_click, move, type, key, scroll, complete, error), but the action system supports 14 action types. The `from_tool_use()` parser in `action.rs` already handles 12 of them (all except batch and wait_for_element). This means Anthropic users cannot access drag, triple_click, right_click, wait, wait_for_element, or batch via native tool_use — they're only available through JSON text parsing on non-Anthropic providers.

## Implementation

1. **`src-tauri/src/llm/provider.rs`** — Add 6 new `Tool` entries to the `build_tools()` function:

   - `drag` — with start_x, start_y, end_x, end_y, button (optional, default "left"), duration_ms (optional, default 500)
   - `triple_click` — with x, y
   - `right_click` — with x, y
   - `wait` — with duration_ms (optional, default 1000)
   - `wait_for_element` — with description, timeout_ms (optional, default 10000, max 10000)
   - `batch` — with actions array (max 10 items, no nesting allowed)

   Use the existing tool definitions and the `Action` enum fields in `action.rs:30-95` as the source of truth for field names, types, and defaults.

2. **`src-tauri/src/llm/provider.rs`** — Update `build_system_prompt_for_tools()` to mention additional capabilities (drag, right-click, triple-click, wait, batch) so the LLM knows they're available.

3. **`src-tauri/src/llm/anthropic.rs`** — Override `supports_tools()` to return `true`:
   ```rust
   fn supports_tools(&self) -> bool {
       true
   }
   ```

4. **`src-tauri/src/agent/action.rs`** — Add `wait_for_element` and `batch` handling to `from_tool_use()` if not already present (currently handles 12 of 14 types; verify and add any missing).

5. **Tests** — Add/update tests in `provider.rs` test module:
   - Verify `build_tools()` returns 14 tools
   - Verify each tool has valid JSON schema with required fields
   - Verify tool names match `from_tool_use()` expected names

## Acceptance Criteria

- [ ] `build_tools()` returns 14 tool definitions (currently 8)
- [ ] Each new tool's JSON schema matches the corresponding `Action` enum variant's fields
- [ ] `from_tool_use()` handles all 14 action types (add batch/wait_for_element if missing)
- [ ] `supports_tools()` returns `true` for AnthropicProvider
- [ ] `build_system_prompt_for_tools()` references new capabilities
- [ ] All existing tests pass (`cargo test`)
- [ ] New tests verify all 14 tool definitions

## Files to Create/Modify

- `src-tauri/src/llm/provider.rs` — Add 6 tool definitions to `build_tools()`, update `build_system_prompt_for_tools()`
- `src-tauri/src/llm/anthropic.rs` — Override `supports_tools()`
- `src-tauri/src/agent/action.rs` — Add batch/wait_for_element to `from_tool_use()` if missing

## Integration Points

- **Provides**: Complete set of 14 native tool definitions for Anthropic provider
- **Consumes**: `Action` enum field definitions from `action.rs`
- **Conflicts**: Avoid modifying `history_to_messages()` or conversation format (handled by fix-anthropic-tool-flow task)
