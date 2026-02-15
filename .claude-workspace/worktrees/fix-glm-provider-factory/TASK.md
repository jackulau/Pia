---
id: fix-glm-provider-factory
name: Add GLM to create_provider_from_config factory
wave: 1
priority: 1
dependencies: []
estimated_hours: 1
tags: [backend, glm, bugfix]
---

## Objective

Add the missing GLM case to `create_provider_from_config()` in `lib.rs` so that health checks and model listing work for the GLM provider.

## Context

The GLM provider is correctly handled in `agent/loop_runner.rs` (line 164) but is **missing** from the `create_provider_from_config()` factory function in `lib.rs` (line 828). This means:
- `check_provider_health("glm")` returns "Unknown provider: glm"
- `list_provider_models("glm")` returns "Unknown provider: glm"
- The settings UI cannot verify GLM connectivity

The fix is straightforward: add a `"glm"` match arm following the exact same pattern used in `loop_runner.rs`.

## Implementation

1. In `src-tauri/src/lib.rs`, add a `"glm"` case to `create_provider_from_config()` (after the "openrouter" case, before "openai-compatible"):
   ```rust
   "glm" => {
       let cfg = config
           .providers
           .glm
           .as_ref()
           .ok_or("GLM not configured")?;
       Ok(Box::new(GlmProvider::new(
           cfg.api_key.clone(),
           cfg.model.clone(),
       )))
   }
   ```
2. Ensure `GlmProvider` is imported at the top of `lib.rs` (check existing imports)
3. Verify with `cargo build`

## Acceptance Criteria

- [ ] `create_provider_from_config("glm", &config)` returns a GlmProvider when GLM is configured
- [ ] `create_provider_from_config("glm", &config)` returns "GLM not configured" error when GLM is not configured
- [ ] `cargo build` succeeds with no new warnings
- [ ] The `"glm"` case matches the pattern used for other providers

## Files to Create/Modify

- `src-tauri/src/lib.rs` - Add GLM match arm to `create_provider_from_config()` (~line 876, before the openai-compatible case)

## Integration Points

- **Provides**: GLM support in the factory function used by `check_provider_health` and `list_provider_models` Tauri commands
- **Consumes**: `GlmProvider` from `llm/glm.rs`, `GlmConfig` from `config/settings.rs`
- **Conflicts**: None - only adds a new match arm to an existing function
