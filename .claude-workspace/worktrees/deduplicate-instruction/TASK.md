---
id: deduplicate-instruction
name: Stop repeating instruction in every conversation message
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [backend, token-optimization, llm]
---

## Objective

Eliminate the redundant repetition of the user instruction in every single conversation message, sending it only once.

## Context

In `loop_runner.rs:314`, every iteration of the agent loop adds a new user message containing the same instruction text:

```rust
conversation.add_user_message(
    &instruction,  // Same instruction every time!
    Some(screenshot.base64.clone()),
    Some(screenshot.width),
    Some(screenshot.height),
);
```

This means in a 20-iteration task, the instruction like "Open Chrome and navigate to gmail.com" is sent 20 times. Each repetition wastes tokens.

Additionally, in each provider's `send_with_history()`, the text for user messages with screenshots gets wrapped:
- Anthropic: `"User instruction: {text}\n\nAnalyze the screenshot and respond with a single JSON action."`
- OpenAI: `"User instruction: {text}\n\nAnalyze the screenshot and respond with a single JSON action."`
- Ollama: `"[Screenshot attached]\n{text}\n\nAnalyze the screenshot and respond with a single JSON action."`

This wrapper text is ALSO repeated every message.

## Implementation

1. **Modify `loop_runner.rs`** - For the first iteration, include the full instruction. For subsequent iterations, use a short continuation message like `"Continue"` or just send the screenshot with no text instruction.

   ```rust
   let user_text = if iteration == 1 {
       instruction.to_string()
   } else {
       "Continue.".to_string()
   };
   conversation.add_user_message(
       &user_text,
       Some(screenshot.base64.clone()),
       Some(screenshot.width),
       Some(screenshot.height),
   );
   ```

2. **Include instruction in system prompt** - Modify the system prompt construction to embed the original instruction, so the LLM always has context. Update `build_system_prompt()` and `build_system_prompt_for_tools()` to accept an optional instruction parameter, OR include it via `conversation.original_instruction()`.

3. **Update provider implementations** - Each provider's `send_with_history()` should include the original instruction in the system prompt. The `ConversationHistory` already has `original_instruction()` available.

4. **Simplify wrapper text in providers** - For subsequent messages (non-first), don't add the "User instruction: ... Analyze the screenshot..." wrapper. Just send the screenshot with minimal text.

## Acceptance Criteria

- [ ] Instruction text is only included in the first user message or system prompt
- [ ] Subsequent iterations use "Continue." or similar short text
- [ ] The original instruction is still accessible to the LLM (via system prompt or first message preservation)
- [ ] All existing tests pass
- [ ] Conversation truncation still preserves first message (already implemented in `truncate_to_max()`)
- [ ] Provider wrapper text is only added to the first message with screenshot

## Files to Create/Modify

- `src-tauri/src/agent/loop_runner.rs` - Change how instruction is added per-iteration (line ~314)
- `src-tauri/src/llm/provider.rs` - Optionally embed instruction in system prompt builders
- `src-tauri/src/llm/anthropic.rs` - Simplify wrapper text for non-first messages
- `src-tauri/src/llm/openai.rs` - Simplify wrapper text for non-first messages
- `src-tauri/src/llm/ollama.rs` - Simplify wrapper text for non-first messages
- `src-tauri/src/llm/openrouter.rs` - Simplify wrapper text for non-first messages
- `src-tauri/src/llm/glm.rs` - Simplify wrapper text for non-first messages
- `src-tauri/src/llm/openai_compatible.rs` - Simplify wrapper text for non-first messages

## Integration Points

- **Provides**: Significant reduction in repeated instruction tokens
- **Consumes**: ConversationHistory.original_instruction()
- **Conflicts**: Modifies provider files - coordinate with compact-system-prompt task for `build_system_prompt()` changes
