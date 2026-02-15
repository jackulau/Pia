---
id: fix-glm-config-scanning
name: Add config file scanning for GLM provider
wave: 1
priority: 2
dependencies: []
estimated_hours: 2
tags: [backend, glm, detection]
---

## Objective

Add CLI config file scanning for the GLM provider in `scan_config_files()` so that GLM API keys can be detected from common tool configurations, matching the detection coverage of other providers.

## Context

Currently, GLM detection (`detect_glm()` in `credentials.rs`) only checks:
1. Environment variables (`GLM_API_KEY`, `ZHIPUAI_API_KEY`, `GLM_KEY`)
2. Dotenv files and shell RC files (via `scan_file_sources()`)

It does NOT scan CLI config files, unlike Anthropic (aider, litellm, Claude CLI), OpenAI (openai auth, copilot, aider), and OpenRouter (aider). The `scan_config_files()` function has an empty `_ => {}` fallback that silently skips GLM.

Zhipu AI keys could appear in:
- **LiteLLM config** (`~/.config/litellm/config.yaml`) with key `api_key` (need to validate it looks like a GLM key - contains `.`)
- **Aider config** (`~/.config/aider/.aider.conf.yml` or `~/.aider.conf.yml`) - aider doesn't have a dedicated GLM key field, but users may configure it via litellm proxy settings

## Implementation

1. In `src-tauri/src/config/credentials.rs`, add a `"glm"` case to `scan_config_files()` (around line 874, before the `_ => {}` fallback):
   ```rust
   "glm" => {
       // LiteLLM config: ~/.config/litellm/config.yaml
       let litellm_path = home.join(".config/litellm/config.yaml");
       if let Some(key) = read_key_from_yaml(&litellm_path, "api_key") {
           // Only include if it looks like a GLM key (JWT-like with '.')
           if key.contains('.') && !key.starts_with("sk-") {
               results.push((key, format!("config:{}", litellm_path.display())));
           }
       }
   }
   ```

2. In `detect_glm()`, add a step 2 to check config files (after env/file source check, before returning None):
   ```rust
   // 2. Check CLI tool config files
   if let Some((key, source)) = scan_config_files("glm").into_iter().next() {
       return Some(DetectedCredential {
           provider: "glm".to_string(),
           api_key: key,
           source,
           model_hint: Some("glm-4v-flash".to_string()),
           host: None,
           available_models: None,
       });
   }
   ```

3. Add tests for the new GLM config file scanning
4. Run `cargo test` to verify all existing tests still pass

## Acceptance Criteria

- [ ] `scan_config_files("glm")` returns keys found in LiteLLM config
- [ ] GLM key detection properly validates the key format (contains `.`, not `sk-` prefixed)
- [ ] `detect_glm()` checks config files as a fallback after env vars
- [ ] All existing tests pass
- [ ] New tests cover the GLM config scanning path
- [ ] `cargo build` succeeds

## Files to Create/Modify

- `src-tauri/src/config/credentials.rs` - Add `"glm"` case to `scan_config_files()` (~line 874) and add config file fallback to `detect_glm()` (~line 607)

## Integration Points

- **Provides**: GLM config file detection in the credential scanning pipeline
- **Consumes**: `read_key_from_yaml()` helper, `validate_key_format()` for GLM
- **Conflicts**: Avoid editing the Ollama section (~line 670) or other provider detection functions - only touch GLM-related code and `scan_config_files()`
