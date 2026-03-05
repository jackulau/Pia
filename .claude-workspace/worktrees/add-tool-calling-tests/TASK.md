---
id: add-tool-calling-tests
name: Add comprehensive tests for multi-turn tool calling pipeline
wave: 3
priority: 2
dependencies: [fix-anthropic-tool-flow]
estimated_hours: 3
tags: [backend, testing]
---

## Objective

Add comprehensive tests verifying the complete tool calling pipeline works correctly for multi-turn Anthropic conversations.

## Context

After the `fix-anthropic-tool-flow` task completes, the tool calling system will be functional, but there are no integration-level tests verifying the end-to-end flow. The existing tests cover individual action parsing and tool definitions in isolation. We need tests that verify:
- Multi-turn conversation formatting for Anthropic
- tool_use_id threading from assistant to tool_result
- Proper content block serialization
- All 14 tool definitions are complete and match action parsing

## Implementation

### 1. `src-tauri/src/llm/anthropic.rs` — Add Anthropic message formatting tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assistant_tool_use_serializes_correctly() {
        // Build an AnthropicMessage with ToolUse content
        // Verify JSON output has type: "tool_use", id, name, input
    }

    #[test]
    fn test_tool_result_serializes_correctly() {
        // Build an AnthropicMessage with ToolResult content
        // Verify JSON output has type: "tool_result", tool_use_id, content
    }

    #[test]
    fn test_multi_turn_message_sequence() {
        // Create a ConversationHistory with:
        //   1. User message with screenshot
        //   2. AssistantToolUse (click)
        //   3. ToolResult with tool_use_id
        //   4. User message with screenshot
        //   5. AssistantToolUse (type)
        //   6. ToolResult with tool_use_id
        // Convert to AnthropicMessages and verify structure
    }

    #[test]
    fn test_tool_result_error_serializes_with_is_error() {
        // Verify failed tool results include is_error: true
    }

    #[test]
    fn test_mixed_text_and_tool_use_assistant_message() {
        // Test AssistantToolUse with both text and tool_use content
    }
}
```

### 2. `src-tauri/src/llm/provider.rs` — Add tool definition completeness tests

```rust
#[test]
fn test_build_tools_returns_14_definitions() {
    let tools = build_tools();
    assert_eq!(tools.len(), 14);
}

#[test]
fn test_all_tools_have_matching_from_tool_use_handler() {
    // For each tool in build_tools(), create a ToolUse with valid input
    // and verify from_tool_use() succeeds
}

#[test]
fn test_tool_names_match_action_variants() {
    // Verify every tool name in build_tools() corresponds to an Action variant
}

#[test]
fn test_history_to_messages_handles_assistant_tool_use() {
    // Create history with AssistantToolUse message
    // Verify history_to_messages() falls back to text for non-Anthropic
}
```

### 3. `src-tauri/src/agent/conversation.rs` — Add multi-turn conversation tests

```rust
#[test]
fn test_tool_use_id_threading() {
    let mut conv = ConversationHistory::new();
    conv.add_user_message("Click the button", None, None, None);
    conv.add_assistant_tool_use(
        "toolu_123".into(), "click".into(),
        json!({"x": 100, "y": 200}), None,
    );
    conv.add_tool_result_with_id("toolu_123".into(), true, Some("Clicked".into()), None);

    // Verify tool_use_id is preserved in tool result
    match &conv.get_messages()[2] {
        Message::ToolResult { tool_use_id, .. } => {
            assert_eq!(tool_use_id.as_deref(), Some("toolu_123"));
        }
        _ => panic!("Expected ToolResult"),
    }
}

#[test]
fn test_truncation_preserves_tool_use_pairs() {
    // Verify that truncation doesn't split a tool_use/tool_result pair
    // (or at minimum handles it gracefully)
}
```

### 4. `src-tauri/src/agent/action.rs` — Verify all 14 actions parse from tool_use

```rust
#[test]
fn test_from_tool_use_all_14_actions() {
    // Test each action type can be parsed from ToolUse:
    // click, double_click, move, type, key, scroll, drag,
    // triple_click, right_click, wait, wait_for_element,
    // batch, complete, error
}
```

## Acceptance Criteria

- [ ] Tests verify AnthropicMessage serialization includes proper content blocks
- [ ] Tests verify multi-turn tool_use → tool_result → tool_use sequences
- [ ] Tests verify all 14 tool definitions have matching from_tool_use handlers
- [ ] Tests verify history_to_messages handles AssistantToolUse fallback
- [ ] Tests verify tool_use_id threading through conversation
- [ ] All tests pass (`cargo test`)

## Files to Create/Modify

- `src-tauri/src/llm/anthropic.rs` — Add test module with Anthropic-specific tests
- `src-tauri/src/llm/provider.rs` — Add tool completeness tests
- `src-tauri/src/agent/conversation.rs` — Add multi-turn conversation tests
- `src-tauri/src/agent/action.rs` — Add from_tool_use completeness tests

## Integration Points

- **Provides**: Test coverage ensuring tool calling doesn't regress
- **Consumes**: All changes from previous 3 tasks
- **Conflicts**: None (only adds test code)
