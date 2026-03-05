---
id: string-allocation-optimize
name: Optimize String Allocations in LLM Providers
wave: 1
priority: 1
dependencies: []
estimated_hours: 3
tags: [backend, performance, memory]
---

## Objective

Reduce excessive string allocations and cloning in LLM provider implementations to improve memory efficiency and reduce GC pressure.

## Context

The LLM providers have multiple inefficient patterns:
1. Excessive `.to_string()` calls (40+ instances)
2. Inefficient buffer processing that allocates on every SSE event
3. No string pre-allocation for response accumulation
4. Base64 image data cloned unnecessarily

Key locations:
- `src-tauri/src/llm/anthropic.rs` lines 145-183 - Two allocations per SSE event
- `src-tauri/src/llm/openai.rs` lines 143-173 - Same pattern
- `src-tauri/src/llm/openrouter.rs` lines 145-175 - Same pattern
- `src-tauri/src/llm/ollama.rs` - Similar issues

## Implementation

1. **Buffer Processing Optimization**:
   - Replace `buffer[..pos].to_string()` and `buffer = buffer[pos + 2..].to_string()` with efficient string slicing
   - Use `String::drain()` or manual index tracking to avoid allocations
   - Example fix:
     ```rust
     // Before (2 allocations per event)
     let event_str = buffer[..pos].to_string();
     buffer = buffer[pos + 2..].to_string();
     
     // After (0 allocations per event)
     let event_str = &buffer[..pos];
     // Process event_str...
     buffer.drain(..pos + 2);
     ```

2. **Pre-allocate Response Strings**:
   - Use `String::with_capacity()` for `full_response` accumulation
   - Estimate capacity based on typical response sizes (~4KB)

3. **Reduce Unnecessary Cloning**:
   - Pass `&str` references instead of owned Strings where possible
   - Use `Cow<str>` for strings that may or may not need ownership

4. **Cache System Prompt**:
   - System prompt is rebuilt every iteration with same screen dimensions
   - Cache it or use lazy_static for the template

## Acceptance Criteria

- [ ] Buffer processing uses zero-allocation slicing
- [ ] Response strings are pre-allocated with capacity hints
- [ ] No unnecessary `.to_string()` or `.clone()` on string data
- [ ] Code compiles without errors
- [ ] LLM streaming still works correctly with all providers

## Files to Create/Modify

- `src-tauri/src/llm/anthropic.rs` - Optimize buffer processing and allocations
- `src-tauri/src/llm/openai.rs` - Same optimizations
- `src-tauri/src/llm/openrouter.rs` - Same optimizations
- `src-tauri/src/llm/ollama.rs` - Same optimizations

## Integration Points

- **Provides**: Optimized LLM response handling
- **Consumes**: LlmProvider trait interface
- **Conflicts**: None - internal implementation changes only
