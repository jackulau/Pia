---
id: fix-glm-health-check
name: Implement health_check and list_models for GLM provider
wave: 1
priority: 1
dependencies: []
estimated_hours: 2
tags: [backend, glm, feature]
---

## Objective

Implement the `health_check()` and `list_models()` trait methods for `GlmProvider` so that GLM provider connectivity and model listing work in the settings UI.

## Context

Currently, `GlmProvider` in `src-tauri/src/llm/glm.rs` does NOT implement `health_check()` or `list_models()`. The `LlmProvider` trait has default implementations that return `Err(LlmError::NotConfigured)`. This means even when GLM is properly configured with a valid API key, the health check in settings always fails.

The GLM (Zhipu AI) API base URL is `https://open.bigmodel.cn/api/paas/v4/`. The chat completions endpoint is at `/chat/completions`. GLM uses an OpenAI-compatible API format, so `GET /models` should work similarly.

Reference: The existing `send_with_history()` method (line 141) already uses the correct base URL and auth header pattern:
- URL: `https://open.bigmodel.cn/api/paas/v4/chat/completions`
- Auth: `Bearer {api_key}`

## Implementation

1. In `src-tauri/src/llm/glm.rs`, implement `health_check()`:
   ```rust
   async fn health_check(&self) -> Result<bool, LlmError> {
       let response = self.client
           .get("https://open.bigmodel.cn/api/paas/v4/models")
           .header("Authorization", format!("Bearer {}", self.api_key))
           .send()
           .await?;
       Ok(response.status().is_success())
   }
   ```

2. Implement `list_models()`:
   ```rust
   async fn list_models(&self) -> Result<Vec<String>, LlmError> {
       let response = self.client
           .get("https://open.bigmodel.cn/api/paas/v4/models")
           .header("Authorization", format!("Bearer {}", self.api_key))
           .send()
           .await?;
       if !response.status().is_success() {
           return Err(LlmError::ApiError(response.text().await.unwrap_or_default()));
       }
       let body: serde_json::Value = response.json().await
           .map_err(|e| LlmError::ApiError(e.to_string()))?;
       let models = body["data"]
           .as_array()
           .map(|arr| arr.iter().filter_map(|m| m["id"].as_str().map(String::from)).collect())
           .unwrap_or_default();
       Ok(models)
   }
   ```

3. Verify the GLM API docs to confirm the models endpoint exists and uses `data[].id` format
4. Run `cargo build` to verify

## Acceptance Criteria

- [ ] `health_check()` returns `Ok(true)` when GLM API is reachable with valid key
- [ ] `health_check()` returns `Ok(false)` or appropriate error with invalid key
- [ ] `list_models()` returns a list of available model IDs
- [ ] `list_models()` returns appropriate error on failure
- [ ] `cargo build` succeeds with no new warnings
- [ ] Follows the same pattern as other providers (e.g., `anthropic.rs`, `openai.rs`)

## Files to Create/Modify

- `src-tauri/src/llm/glm.rs` - Add `health_check()` and `list_models()` implementations to the `LlmProvider` impl block

## Integration Points

- **Provides**: Working health check and model listing for GLM provider
- **Consumes**: GLM API at `https://open.bigmodel.cn/api/paas/v4/`
- **Conflicts**: None - only adds new method implementations to existing impl block
