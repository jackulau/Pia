---
id: reduce-max-tokens
name: Reduce hardcoded max_tokens and make it configurable
wave: 1
priority: 3
dependencies: []
estimated_hours: 2
tags: [backend, token-optimization, llm, config]
---

## Objective

Reduce the hardcoded `max_tokens: 1024` to a more efficient default and make it configurable, since action responses are typically only 50-100 tokens.

## Context

Every LLM provider hardcodes `max_tokens: 1024` in their request structs:
- `anthropic.rs:169`: `max_tokens: 1024`
- `openai.rs:126`: `max_tokens: 1024`
- `openrouter.rs:120`: `max_tokens: 1024`
- `glm.rs:134`: `max_tokens: 1024`
- `openai_compatible.rs:132`: `max_tokens: 1024`

Typical action responses are small JSON objects like `{"action":"click","x":100,"y":200}` which are ~20-50 tokens. Even with reasoning text, responses rarely exceed 200 tokens. Setting `max_tokens: 1024` means:
- The LLM reserves 1024 tokens of its context window for output, reducing available input context
- Potential for runaway generation if the LLM goes off-track (wastes output tokens)
- With some APIs, billing includes reserved output tokens

Reducing to 512 (or even 256 for tool-based Anthropic which doesn't need reasoning text) would be more efficient.

## Implementation

1. **Add `max_response_tokens` to `GeneralConfig`** in `config/settings.rs`:
   ```rust
   #[serde(default = "default_max_response_tokens")]
   pub max_response_tokens: u32,
   ```
   Default: `512`

2. **Pass through to providers** - Update the `LlmProvider` trait or provider constructors to accept the max_tokens value instead of hardcoding it.

3. **Update each provider** to use the config value:
   - `anthropic.rs` - Use 512 (or lower since tool_use responses are very compact)
   - `openai.rs` - Use configured value
   - `openrouter.rs` - Use configured value
   - `glm.rs` - Use configured value
   - `openai_compatible.rs` - Use configured value
   - `ollama.rs` - Ollama doesn't use max_tokens in the same way but could add `num_predict` option

4. **Update `loop_runner.rs`** to pass the config value to the provider

## Acceptance Criteria

- [ ] `max_response_tokens` is configurable in settings with default of 512
- [ ] All providers use the configured value instead of hardcoded 1024
- [ ] Existing functionality is not broken (actions still parse correctly with lower limit)
- [ ] All tests pass
- [ ] Settings UI doesn't need changes (config is TOML-based)

## Files to Create/Modify

- `src-tauri/src/config/settings.rs` - Add `max_response_tokens` config field
- `src-tauri/src/llm/anthropic.rs` - Use config value
- `src-tauri/src/llm/openai.rs` - Use config value
- `src-tauri/src/llm/openrouter.rs` - Use config value
- `src-tauri/src/llm/glm.rs` - Use config value
- `src-tauri/src/llm/openai_compatible.rs` - Use config value
- `src-tauri/src/llm/provider.rs` - Update trait or add parameter

## Integration Points

- **Provides**: Reduced output token reservation, configurable max_tokens
- **Consumes**: Config system
- **Conflicts**: Provider files are also touched by deduplicate-instruction task - coordinate changes
