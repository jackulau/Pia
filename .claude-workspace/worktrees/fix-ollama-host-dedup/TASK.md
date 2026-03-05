---
id: fix-ollama-host-dedup
name: Fix Ollama host URL deduplication bug
wave: 1
priority: 2
dependencies: []
estimated_hours: 1
tags: [backend, ollama, bugfix]
---

## Objective

Fix the Ollama host deduplication bug in `detect_ollama()` that uses `.dedup()` (which only removes *consecutive* duplicates) instead of properly deduplicating by normalized URL.

## Context

In `src-tauri/src/config/credentials.rs` at line 689, `detect_ollama()` builds a candidate host list and calls `.dedup()`:

```rust
let mut hosts = Vec::new();
if let Ok(custom_host) = env::var("OLLAMA_HOST") {
    let trimmed = custom_host.trim().trim_end_matches('/').to_string();
    if !trimmed.is_empty() {
        hosts.push(trimmed);
    }
}
hosts.push("http://localhost:11434".to_string());
hosts.push("http://127.0.0.1:11434".to_string());
hosts.dedup();  // BUG: only removes consecutive duplicates
```

**Bug scenarios:**
1. If `OLLAMA_HOST=http://localhost:11434`, the list is `["http://localhost:11434", "http://localhost:11434", "http://127.0.0.1:11434"]` - `.dedup()` works here by coincidence since they're consecutive.
2. If `OLLAMA_HOST=http://127.0.0.1:11434`, the list is `["http://127.0.0.1:11434", "http://localhost:11434", "http://127.0.0.1:11434"]` - `.dedup()` does NOT remove the duplicate because they're not consecutive. This probes the same host twice.
3. More subtly, `localhost` and `127.0.0.1` resolve to the same host but are never deduplicated semantically.

## Implementation

1. Replace `.dedup()` with a proper deduplication that:
   - Uses a `HashSet` or manual dedup to handle non-consecutive duplicates
   - Normalizes URLs before comparison (trim trailing slashes, lowercase)

   Suggested fix:
   ```rust
   // Deduplicate hosts (handles non-consecutive duplicates)
   let mut seen = std::collections::HashSet::new();
   hosts.retain(|h| seen.insert(h.clone()));
   ```

2. Add unit tests for the dedup behavior:
   - Test with OLLAMA_HOST matching `localhost:11434`
   - Test with OLLAMA_HOST matching `127.0.0.1:11434`
   - Test with OLLAMA_HOST being a unique custom host

3. Run `cargo test` to verify

## Acceptance Criteria

- [ ] Duplicate hosts are removed regardless of position in the list
- [ ] Custom OLLAMA_HOST that matches a default is properly deduplicated
- [ ] The fix doesn't change behavior when there are no duplicates
- [ ] `cargo build` and `cargo test` succeed
- [ ] New tests verify dedup behavior

## Files to Create/Modify

- `src-tauri/src/config/credentials.rs` - Fix dedup logic in `detect_ollama()` (~line 689)

## Integration Points

- **Provides**: Correct Ollama host deduplication preventing redundant network probes
- **Consumes**: None
- **Conflicts**: Avoid editing the GLM section or other provider detection functions - only touch `detect_ollama()` and its tests
