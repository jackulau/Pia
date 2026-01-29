---
id: test-openai-provider
name: Verify OpenAI Provider Works Correctly
wave: 1
priority: 1
dependencies: []
estimated_hours: 2
tags: [llm, testing, openai]
---

## Objective

Verify the OpenAI GPT provider implementation works correctly with proper error handling, streaming, and token metrics.

## Context

The OpenAI provider at `src-tauri/src/llm/openai.rs` implements the LlmProvider trait and uses OpenAI Chat Completions API with SSE streaming. This task ensures the implementation handles all edge cases and API responses correctly.

## Implementation

1. Review the OpenAI provider implementation in `src-tauri/src/llm/openai.rs`
2. Verify API request format matches OpenAI's Chat Completions API specification
3. Check SSE streaming response handling:
   - Line-by-line parsing with `data:` prefix
   - `[DONE]` termination signal
   - Delta content extraction from choices array
4. Verify error handling for API errors (invalid key, rate limits, quota exceeded)
5. Ensure the request includes required headers:
   - `Authorization: Bearer {api_key}`
   - `Content-Type: application/json`
6. Verify image format uses data URL encoding (`data:image/png;base64,{data}`)
7. Test with actual API call if API key is available
8. Run `cargo build` to ensure no compilation errors

## Acceptance Criteria

- [ ] API request format matches OpenAI Chat Completions API
- [ ] SSE streaming with `data:` prefix is properly parsed
- [ ] `[DONE]` signal is correctly handled
- [ ] Error responses return appropriate LlmError variants
- [ ] Token metrics (prompt_tokens, completion_tokens) are correctly extracted
- [ ] Image encoding uses proper data URL format
- [ ] Code compiles without errors
- [ ] Streaming callback properly invokes on_chunk

## Files to Review/Modify

- `src-tauri/src/llm/openai.rs` - OpenAI provider implementation
- `src-tauri/src/llm/provider.rs` - LlmProvider trait definition

## Integration Points

- **Provides**: Verified OpenAI provider for agent loop
- **Consumes**: LlmProvider trait interface
- **Conflicts**: None
