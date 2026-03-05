---
id: test-anthropic-provider
name: Verify Anthropic Provider Works Correctly
wave: 1
priority: 1
dependencies: []
estimated_hours: 2
tags: [llm, testing, anthropic]
---

## Objective

Verify the Anthropic Claude provider implementation works correctly with proper error handling, streaming, and token metrics.

## Context

The Anthropic provider at `src-tauri/src/llm/anthropic.rs` implements the LlmProvider trait and uses Claude API with SSE streaming. This task ensures the implementation handles all edge cases and API responses correctly.

## Implementation

1. Review the Anthropic provider implementation in `src-tauri/src/llm/anthropic.rs`
2. Verify API request format matches Anthropic's Messages API specification
3. Check SSE streaming event handling for all event types:
   - `message_start` - input token counting
   - `content_block_delta` - text content streaming
   - `message_delta` - output token counting
4. Verify error handling for API errors (invalid key, rate limits, etc.)
5. Ensure the request includes required headers:
   - `x-api-key` header for authentication
   - `anthropic-version: 2023-06-01`
   - `content-type: application/json`
6. Test with actual API call if API key is available
7. Run `cargo build` to ensure no compilation errors

## Acceptance Criteria

- [ ] API request format matches Anthropic Messages API v2023-06-01
- [ ] All SSE event types are properly handled
- [ ] Error responses return appropriate LlmError variants
- [ ] Token metrics (input_tokens, output_tokens) are correctly extracted
- [ ] Image encoding format is correct (base64 with media_type)
- [ ] Code compiles without errors
- [ ] Streaming callback properly invokes on_chunk

## Files to Review/Modify

- `src-tauri/src/llm/anthropic.rs` - Anthropic provider implementation
- `src-tauri/src/llm/provider.rs` - LlmProvider trait definition

## Integration Points

- **Provides**: Verified Anthropic provider for agent loop
- **Consumes**: LlmProvider trait interface
- **Conflicts**: None
