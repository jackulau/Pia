---
id: test-openrouter-provider
name: Verify OpenRouter Provider Works Correctly
wave: 1
priority: 1
dependencies: []
estimated_hours: 2
tags: [llm, testing, openrouter]
---

## Objective

Verify the OpenRouter provider implementation works correctly with proper error handling, streaming, and token metrics.

## Context

The OpenRouter provider at `src-tauri/src/llm/openrouter.rs` implements the LlmProvider trait and uses OpenRouter's OpenAI-compatible API with additional required headers. This allows access to multiple model providers through a single API.

## Implementation

1. Review the OpenRouter provider implementation in `src-tauri/src/llm/openrouter.rs`
2. Verify API request format matches OpenRouter's OpenAI-compatible API
3. Check required OpenRouter-specific headers:
   - `Authorization: Bearer {api_key}`
   - `Content-Type: application/json`
   - `HTTP-Referer` - required for API access
   - `X-Title` - application identification
4. Check SSE streaming response handling (same as OpenAI format):
   - Line-by-line parsing with `data:` prefix
   - `[DONE]` termination signal
   - Delta content extraction from choices array
5. Verify error handling for API errors (invalid key, model not found, etc.)
6. Verify model naming convention (e.g., `anthropic/claude-sonnet-4-20250514`)
7. Test with actual API call if API key is available
8. Run `cargo build` to ensure no compilation errors

## Acceptance Criteria

- [ ] API request format matches OpenRouter OpenAI-compatible API
- [ ] Required headers (HTTP-Referer, X-Title) are included
- [ ] SSE streaming with `data:` prefix is properly parsed
- [ ] `[DONE]` signal is correctly handled
- [ ] Error responses return appropriate LlmError variants
- [ ] Token metrics (prompt_tokens, completion_tokens) are correctly extracted
- [ ] Image encoding uses proper data URL format
- [ ] Model naming supports provider prefix (e.g., `anthropic/claude-*`)
- [ ] Code compiles without errors
- [ ] Streaming callback properly invokes on_chunk

## Files to Review/Modify

- `src-tauri/src/llm/openrouter.rs` - OpenRouter provider implementation
- `src-tauri/src/llm/provider.rs` - LlmProvider trait definition

## Integration Points

- **Provides**: Verified OpenRouter provider for multi-model access
- **Consumes**: LlmProvider trait interface
- **Conflicts**: None
