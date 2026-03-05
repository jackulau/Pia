---
id: xplat-llm-platform-prompt
name: Make LLM system prompts platform-aware
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [backend, llm, cross-platform, P1]
---

## Objective

Make the system prompt sent to LLMs platform-aware so the model generates correct keyboard shortcuts and platform-appropriate actions for the current OS.

## Context

In `src-tauri/src/llm/provider.rs`, the system prompt and tool definitions tell the LLM about available actions and keyboard modifiers. Currently:

- Line ~180: `"meta is cmd on macOS"` — informational but doesn't tell the LLM which OS is active
- Line ~575: `Available modifiers: "ctrl", "alt", "shift", "meta" (cmd on macOS)` — same issue
- The LLM may generate `Key { modifiers: ["cmd"] }` on Windows because it doesn't know the platform

If the LLM generates macOS shortcuts on Windows (or vice versa), the agent will perform wrong actions.

## Implementation Steps

1. **Add a platform detection function** that returns a string the LLM can understand:
   ```rust
   fn current_platform() -> &'static str {
       if cfg!(target_os = "macos") { "macOS" }
       else if cfg!(target_os = "windows") { "Windows" }
       else { "Linux" }
   }
   ```

2. **Inject platform info into the system prompt** in `build_system_prompt()` (provider.rs). Add a section like:
   ```
   You are controlling a {platform} computer.
   - On macOS: use "cmd" modifier for shortcuts (Cmd+C, Cmd+V, Cmd+Z, etc.)
   - On Windows/Linux: use "ctrl" modifier for shortcuts (Ctrl+C, Ctrl+V, Ctrl+Z, etc.)
   - The current platform is: {platform}
   ```

3. **Update tool descriptions** for the `key` tool to include platform-specific guidance in the modifier description.

4. **Thread the platform string** through `build_system_prompt()` — it currently takes no platform parameter. Either:
   - Use `cfg!()` directly in the function (simplest, compile-time)
   - Pass it as a parameter from `AgentLoop` (more testable)

5. **Add tests** that verify:
   - System prompt contains the correct platform name
   - Modifier guidance matches the compile target
   - Tool descriptions include platform-appropriate examples

## Acceptance Criteria

- [ ] System prompt explicitly tells the LLM which platform it's running on
- [ ] Modifier guidance is correct for each platform (macOS→cmd, Windows/Linux→ctrl)
- [ ] All existing tests pass (especially the provider.rs tests)
- [ ] New tests verify platform-aware prompt content

## Files to Create/Modify

- **Modify:** `src-tauri/src/llm/provider.rs` — update `build_system_prompt()` and tool definitions

## Integration Points

- `build_system_prompt()` is called from each LLM provider's `send_with_history()` / `send_with_image()` 
- The system prompt is the first message in every LLM conversation
- Changes here affect ALL providers (Anthropic, OpenAI, Ollama, OpenRouter, GLM, OpenAI-compatible)
