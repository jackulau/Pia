---
id: fix-anthropic-tool-flow
name: Wire up proper Anthropic tool_use/tool_result conversation flow
wave: 2
priority: 1
dependencies: [add-missing-tool-defs, fix-conversation-model]
estimated_hours: 5
tags: [backend, llm, anthropic, critical]
---

## Objective

Wire up the full Anthropic native tool_use/tool_result flow so multi-turn conversations properly format tool_use content blocks in assistant messages and tool_result content blocks in user messages.

## Context

This is the **most critical fix** for making tool calling fully functional. Currently:

1. **Assistant messages are wrong**: When Anthropic returns `LlmResponse::ToolUse(ToolUse { id, name, input })`, the loop stores it via `conversation.add_assistant_message(&serde_json::to_string(tool_use))` — a plain text message. Anthropic expects assistant messages to contain `{ type: "tool_use", id: "toolu_xxx", name: "click", input: {...} }` content blocks.

2. **Tool results are wrong**: After executing an action, `conversation.add_tool_result(true, message, None)` stores a `ToolResult` with no `tool_use_id`. When building Anthropic messages, `history_to_messages()` converts this to a plain user text message like `"Action executed successfully. Clicked left at (100, 200)"`. Anthropic expects `{ type: "tool_result", tool_use_id: "toolu_xxx", content: "..." }` content blocks.

3. **The Anthropic provider uses generic `history_to_messages()`** which has no concept of tool_use/tool_result content blocks — it only produces `(role, text, image)` tuples.

After the `fix-conversation-model` task completes, the `Message` enum will have `AssistantToolUse` and `ToolResult` with `tool_use_id`. This task wires everything together.

## Implementation

### 1. `src-tauri/src/agent/loop_runner.rs` — Use new conversation methods for tool_use

In the main loop (~line 369-371), change how the assistant response is stored:

```rust
// BEFORE:
let response_str = response.to_string_repr();
conversation.add_assistant_message(&response_str);

// AFTER:
match &response {
    LlmResponse::ToolUse(tool_use) => {
        conversation.add_assistant_tool_use(
            tool_use.id.clone(),
            tool_use.name.clone(),
            tool_use.input.clone(),
            None, // no text prefix
        );
    }
    LlmResponse::Text(text) => {
        conversation.add_assistant_message(text);
    }
}
// Keep response_str for logging/history
let response_str = response.to_string_repr();
```

### 2. `src-tauri/src/agent/loop_runner.rs` — Thread tool_use_id to tool results

After action execution (~line 447-448), pass the tool_use_id:

```rust
// Extract tool_use_id from the response
let tool_use_id = match &response {
    LlmResponse::ToolUse(tu) => Some(tu.id.clone()),
    _ => None,
};

// In the success branch:
if let Some(id) = &tool_use_id {
    conversation.add_tool_result_with_id(id.clone(), true, result.message.clone(), None);
} else {
    conversation.add_tool_result(true, result.message.clone(), None);
}

// In the error branch (around line 502):
if let Some(id) = &tool_use_id {
    conversation.add_tool_result_with_id(id.clone(), false, None, Some(error_msg));
} else {
    conversation.add_tool_result(false, None, Some(error_msg));
}
```

### 3. `src-tauri/src/llm/anthropic.rs` — Add ToolUse and ToolResult content variants

Add new `AnthropicContent` variants for proper serialization:

```rust
#[derive(Serialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        content: String,
    },
}
```

### 4. `src-tauri/src/llm/anthropic.rs` — Build messages with native content blocks

Replace the generic `history_to_messages()` call with Anthropic-specific message building that handles `AssistantToolUse` and `ToolResult` with `tool_use_id`:

```rust
// Instead of history_to_messages(), iterate conversation directly:
let messages: Vec<AnthropicMessage> = history.messages()
    .map(|msg| match msg {
        Message::User { instruction, screenshot_base64, .. } => {
            // Same as current: image + text content
            AnthropicMessage { role: "user".into(), content: user_content }
        }
        Message::Assistant { content } => {
            // Plain text assistant (non-tool-use turns)
            AnthropicMessage {
                role: "assistant".into(),
                content: vec![AnthropicContent::Text { text: content.clone() }],
            }
        }
        Message::AssistantToolUse { tool_use_id, tool_name, tool_input, text } => {
            // Native tool_use content block
            let mut content = Vec::new();
            if let Some(t) = text {
                content.push(AnthropicContent::Text { text: t.clone() });
            }
            content.push(AnthropicContent::ToolUse {
                id: tool_use_id.clone(),
                name: tool_name.clone(),
                input: tool_input.clone(),
            });
            AnthropicMessage { role: "assistant".into(), content }
        }
        Message::ToolResult { success, tool_use_id, message, error } => {
            // Native tool_result content block (if tool_use_id present)
            if let Some(id) = tool_use_id {
                let result_text = if *success {
                    message.as_deref().unwrap_or("Action executed successfully").to_string()
                } else {
                    error.as_deref().unwrap_or("Action failed").to_string()
                };
                AnthropicMessage {
                    role: "user".into(),
                    content: vec![AnthropicContent::ToolResult {
                        tool_use_id: id.clone(),
                        is_error: if *success { None } else { Some(true) },
                        content: result_text,
                    }],
                }
            } else {
                // Fallback for tool results without ID (legacy)
                let text = format_tool_result_text(*success, message, error);
                AnthropicMessage {
                    role: "user".into(),
                    content: vec![AnthropicContent::Text { text }],
                }
            }
        }
    })
    .collect();
```

### 5. Ensure `history_to_messages()` in `provider.rs` handles new variants

The generic `history_to_messages()` is still used by non-Anthropic providers. It needs to handle `AssistantToolUse` by falling back to text:

```rust
Message::AssistantToolUse { tool_name, tool_input, text, .. } => {
    // For non-Anthropic providers, serialize as JSON text
    let action_json = serde_json::json!({
        "action": tool_name,
        // spread tool_input fields
    });
    let content = text.as_deref().unwrap_or("").to_string()
        + &serde_json::to_string(&tool_input).unwrap_or_default();
    ("assistant".to_string(), content, None)
}
```

## Acceptance Criteria

- [ ] Anthropic assistant messages contain native `tool_use` content blocks (not text with JSON)
- [ ] Anthropic user messages contain native `tool_result` content blocks with matching `tool_use_id`
- [ ] `tool_use_id` is threaded from LLM response through conversation to tool result
- [ ] Non-Anthropic providers still work correctly (backward compatible fallback)
- [ ] Multi-turn conversations with Anthropic work correctly (tool_use → tool_result → screenshot → tool_use → ...)
- [ ] All existing tests pass (`cargo test`)
- [ ] Preview mode still works (doesn't add tool results for non-executed actions)

## Files to Create/Modify

- `src-tauri/src/agent/loop_runner.rs` — Use `add_assistant_tool_use()` and `add_tool_result_with_id()` for tool_use responses
- `src-tauri/src/llm/anthropic.rs` — Add `ToolUse`/`ToolResult` content variants, build messages with native content blocks
- `src-tauri/src/llm/provider.rs` — Update `history_to_messages()` to handle `AssistantToolUse` variant

## Integration Points

- **Provides**: Fully functional multi-turn native tool calling for Anthropic
- **Consumes**: `Message::AssistantToolUse` and `ToolResult.tool_use_id` from `fix-conversation-model`, tool definitions from `add-missing-tool-defs`
- **Conflicts**: Touches same files as Wave 1 tasks but different sections. Must be merged AFTER Wave 1.
