---
id: compact-system-prompt
name: Optimize system prompts for token efficiency
wave: 1
priority: 2
dependencies: []
estimated_hours: 2
tags: [backend, token-optimization, llm]
---

## Objective

Reduce the token footprint of system prompts sent with every LLM request by making them more concise without losing essential information.

## Context

There are two system prompts in `src-tauri/src/llm/provider.rs`:

1. **`build_system_prompt()`** (lines 407-490) - Used by JSON-based providers (OpenAI, Ollama, OpenRouter, GLM, OpenAI-compatible). This is ~2KB of text listing all 14 action types with verbose examples and guidelines. It's sent as a system message with EVERY request.

2. **`build_system_prompt_for_tools()`** (lines 388-403) - Used by Anthropic (tool-based). Much shorter (~300 bytes) since tool definitions are sent via the API. This is already fairly efficient.

Additionally, `build_tools()` returns 8 tool definitions for Anthropic, but the JSON prompt describes 14 actions. The tool definitions are missing: drag, triple_click, right_click, wait, wait_for_element, and batch. These should be added to `build_tools()` for Anthropic AND removed from the system prompt text (letting the tool schema serve as documentation).

## Implementation

1. **Compact `build_system_prompt()`** in `provider.rs`:
   - Use terse action descriptions (one line each instead of multi-line examples)
   - Remove redundant examples (e.g., don't show both `key` examples, just one)
   - Remove the verbose "Note: Actions are automatically retried..." paragraph
   - Remove "If an action consistently fails, try:" paragraph - the LLM will figure this out from conversation context
   - Target: reduce from ~2KB to ~800 bytes

2. **Add missing tools to `build_tools()`** in `provider.rs`:
   - Add: `drag`, `triple_click`, `right_click`, `wait`, `wait_for_element`, `batch` (6 new tools)
   - This lets Anthropic use all 14 actions natively via tool_use

3. **Update tests** to reflect new tool count and compact prompt

## Acceptance Criteria

- [ ] `build_system_prompt()` is at least 50% shorter while preserving all action types
- [ ] `build_tools()` includes all 14 action types (currently only 8)
- [ ] Prompt still includes screen dimensions
- [ ] All existing tests pass (update assertions for new tool count)
- [ ] New tests verify all actions are documented in both prompt formats
- [ ] JSON response format instructions are preserved for JSON-based providers

## Files to Create/Modify

- `src-tauri/src/llm/provider.rs` - Compact `build_system_prompt()`, expand `build_tools()`
- `src-tauri/src/llm/provider.rs` - Update test assertions

## Integration Points

- **Provides**: Reduced system prompt tokens for all providers
- **Consumes**: None
- **Conflicts**: Avoid editing provider-specific files (anthropic.rs, openai.rs, etc.)
