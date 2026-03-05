---
id: test-ollama-provider
name: Verify Ollama Provider Works Correctly
wave: 1
priority: 1
dependencies: []
estimated_hours: 2
tags: [llm, testing, ollama]
---

## Objective

Verify the Ollama provider implementation works correctly for local model inference with proper streaming and token metrics.

## Context

The Ollama provider at `src-tauri/src/llm/ollama.rs` implements the LlmProvider trait and uses Ollama's local API. This provider is unique as it doesn't require API keys and uses a different request format (generate endpoint vs chat endpoint).

## Implementation

1. Review the Ollama provider implementation in `src-tauri/src/llm/ollama.rs`
2. Verify API request format matches Ollama's Generate API:
   - Uses `/api/generate` endpoint
   - Request contains: model, prompt, images array, stream
   - No authentication required
3. Check streaming response handling:
   - Line-by-line JSON parsing (newline-delimited JSON)
   - Extract `response` field for content
   - Handle `done: true` for completion
4. Verify token metrics extraction:
   - `eval_count` for output tokens
   - `prompt_eval_count` for input tokens
5. Verify image format is raw base64 (no data URL prefix)
6. Test connection to localhost:11434 if Ollama is running
7. Run `cargo build` to ensure no compilation errors

## Acceptance Criteria

- [ ] API request format matches Ollama Generate API
- [ ] Uses correct endpoint: `{host}/api/generate`
- [ ] Streaming NDJSON responses are properly parsed
- [ ] Error responses return appropriate LlmError variants
- [ ] Token metrics (eval_count, prompt_eval_count) are correctly extracted
- [ ] Image encoding is raw base64 (no data URL wrapper)
- [ ] Configurable host (default: http://localhost:11434)
- [ ] Code compiles without errors
- [ ] Streaming callback properly invokes on_chunk

## Files to Review/Modify

- `src-tauri/src/llm/ollama.rs` - Ollama provider implementation
- `src-tauri/src/llm/provider.rs` - LlmProvider trait definition

## Integration Points

- **Provides**: Verified Ollama provider for local model inference
- **Consumes**: LlmProvider trait interface
- **Conflicts**: None
