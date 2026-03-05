---
id: screenshot-change-detection
name: Skip sending unchanged screenshots to save tokens
wave: 2
priority: 3
dependencies: [strip-old-screenshots]
estimated_hours: 3
tags: [backend, token-optimization, llm, capture]
---

## Objective

Detect when the screen hasn't changed between iterations and skip sending the duplicate screenshot, saving significant image tokens.

## Context

In `loop_runner.rs`, every iteration captures a fresh screenshot and sends it to the LLM. But many actions (like `wait`, `key` press for non-visible keys, `move`) may not produce visible screen changes. In these cases, the same (or nearly identical) screenshot is sent again, wasting image tokens.

The `retry.rs` module already has `RetryContext::screen_changed()` which compares screenshots. This logic can be reused.

Image tokens are especially expensive:
- Anthropic: Images are tokenized at ~1600 tokens per 1024x768 tile
- OpenAI: Similar pricing for image inputs
- Ollama: Processing time for image encoding

## Implementation

1. **Add screenshot comparison to `loop_runner.rs`**:
   - Store the hash (or a quick fingerprint) of the last sent screenshot
   - Before adding a user message, compare the new screenshot with the last one
   - If they're identical (or very similar), send the user message WITHOUT a screenshot, with text like `"[Screen unchanged] Continue."`
   - If they're different, send normally with the screenshot

2. **Add a fast comparison method** to `src-tauri/src/capture/mod.rs` or a utility:
   - Use a simple hash (e.g., hash first 1KB + last 1KB + length of base64 string) for fast comparison
   - Don't need pixel-perfect comparison, just a quick check

3. **Track last screenshot hash** in the agent loop:
   ```rust
   let mut last_screenshot_hash: Option<u64> = None;

   // In the loop:
   let current_hash = quick_hash(&screenshot.base64);
   let screen_changed = last_screenshot_hash.map_or(true, |h| h != current_hash);

   if screen_changed {
       conversation.add_user_message(&user_text, Some(screenshot.base64.clone()), ...);
       last_screenshot_hash = Some(current_hash);
   } else {
       conversation.add_user_message("[Screen unchanged] Continue.", None, None, None);
   }
   ```

4. **Always send screenshot on first iteration** and after errors

## Acceptance Criteria

- [ ] Unchanged screenshots are detected and not resent
- [ ] First iteration always includes a screenshot
- [ ] Iterations after errors always include a screenshot
- [ ] The LLM is informed when the screen is unchanged
- [ ] Hash comparison is fast (<1ms) and doesn't slow down the loop
- [ ] All existing tests pass
- [ ] New tests verify screenshot dedup logic

## Files to Create/Modify

- `src-tauri/src/agent/loop_runner.rs` - Add screenshot hash tracking and conditional sending
- `src-tauri/src/capture/mod.rs` - Add quick_hash utility (or inline in loop_runner)

## Integration Points

- **Provides**: Skips redundant screenshot tokens when screen is unchanged
- **Consumes**: Screenshot from capture module
- **Conflicts**: Coordinate with `strip-old-screenshots` task (both modify how screenshots enter conversation)
- **Depends on**: `strip-old-screenshots` should be merged first to establish the pattern for screenshot handling
