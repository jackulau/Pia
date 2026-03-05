---
id: strip-old-screenshots
name: Strip screenshots from older conversation history messages
wave: 1
priority: 1
dependencies: []
estimated_hours: 3
tags: [backend, token-optimization, llm]
---

## Objective

Strip base64 screenshot data from older conversation messages to dramatically reduce input token count per LLM call.

## Context

This is the single biggest token optimization opportunity. Each screenshot is ~1-2MB base64 encoded, which translates to thousands of input tokens. The conversation history keeps up to 20 messages (`MAX_HISTORY_LENGTH` in `conversation.rs`), and every user message includes a screenshot. This means up to ~10 screenshots are being sent with every LLM request, when only the most recent 1-2 are actually needed for the LLM to understand the current screen state.

Currently in `loop_runner.rs:314`, every iteration adds a new user message with the full screenshot. When the history is converted to provider messages via `history_to_messages()` in `provider.rs:351`, ALL screenshots are included.

## Implementation

1. **Modify `src-tauri/src/llm/provider.rs`** - Update `history_to_messages()` to only include screenshots for the most recent N user messages (default N=2). For older messages, replace the screenshot with `None` and prefix the text with `[Screenshot omitted - see latest]`.

2. **Add a configurable constant** in `provider.rs`:
   ```rust
   /// Number of recent screenshots to include in conversation history.
   /// Older screenshots are stripped to save tokens.
   const MAX_SCREENSHOTS_IN_HISTORY: usize = 2;
   ```

3. **Update the function signature** of `history_to_messages()` to count user messages with screenshots from the end, and only pass through the last `MAX_SCREENSHOTS_IN_HISTORY` screenshots.

4. **Update tests** in `provider.rs` to verify screenshot stripping behavior.

## Acceptance Criteria

- [ ] Only the most recent 2 user messages include screenshots in LLM requests
- [ ] Older messages have screenshots replaced with None (no image sent)
- [ ] Text content of older messages is preserved with a note that screenshot was omitted
- [ ] All existing tests pass
- [ ] New tests verify screenshot stripping with >2 user messages
- [ ] No changes to conversation.rs storage (screenshots are still stored for history/export)

## Files to Create/Modify

- `src-tauri/src/llm/provider.rs` - Modify `history_to_messages()` to strip old screenshots
- `src-tauri/src/llm/provider.rs` - Add tests for new behavior

## Integration Points

- **Provides**: Reduced input token count for all LLM providers
- **Consumes**: ConversationHistory (read-only, no changes to storage)
- **Conflicts**: None - only modifies the message conversion layer, not storage
