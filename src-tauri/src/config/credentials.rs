use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Metadata about a detected credential (safe to send to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedCredentialPayload {
    pub provider: String,
    pub source: String,
    pub model_hint: Option<String>,
    pub key_preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_models: Option<Vec<String>>,
}

/// Internal struct with full API key (never sent to frontend)
#[derive(Debug, Clone)]
pub struct DetectedCredential {
    pub provider: String,
    pub api_key: String,
    pub source: String,
    pub model_hint: Option<String>,
    pub host: Option<String>,
    pub available_models: Option<Vec<String>>,
}

impl DetectedCredential {
    pub fn to_payload(&self) -> DetectedCredentialPayload {
        let key_preview = mask_key(&self.api_key);
        DetectedCredentialPayload {
            provider: self.provider.clone(),
            source: self.source.clone(),
            model_hint: self.model_hint.clone(),
            key_preview,
            host: self.host.clone(),
            available_models: self.available_models.clone(),
        }
    }
}

/// Mask an API key for safe display: show first 4 and last 4 chars
fn mask_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.len() <= 8 {
        return "*".repeat(trimmed.len());
    }
    let prefix = &trimmed[..4];
    let suffix = &trimmed[trimmed.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

/// Detect all available credentials from environment variables, CLI config files, and running services.
pub async fn detect_all_credentials() -> Vec<DetectedCredential> {
    let mut results = Vec::new();

    if let Some(cred) = detect_anthropic() {
        results.push(cred);
    }
    if let Some(cred) = detect_openai() {
        results.push(cred);
    }
    if let Some(cred) = detect_openrouter() {
        results.push(cred);
    }
    if let Some(cred) = detect_glm() {
        results.push(cred);
    }
    if let Some(cred) = detect_ollama().await {
        results.push(cred);
    }

    results
}

/// Detect credential for a single provider by name.
pub async fn detect_credential(provider: &str) -> Option<DetectedCredential> {
    match provider {
        "anthropic" => detect_anthropic(),
        "openai" => detect_openai(),
        "openrouter" => detect_openrouter(),
        "glm" => detect_glm(),
        "ollama" => detect_ollama().await,
        _ => None,
    }
}

fn detect_anthropic() -> Option<DetectedCredential> {
    // 1. Check environment variable
    if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
        if !key.trim().is_empty() {
            return Some(DetectedCredential {
                provider: "anthropic".to_string(),
                api_key: key.trim().to_string(),
                source: "env:ANTHROPIC_API_KEY".to_string(),
                model_hint: Some("claude-sonnet-4-20250514".to_string()),
                host: None,
                available_models: None,
            });
        }
    }

    // 2. Check Claude Code config (~/.claude.json or ~/.claude/config.json)
    if let Some(key) = read_claude_code_key() {
        return Some(DetectedCredential {
            provider: "anthropic".to_string(),
            api_key: key,
            source: "claude-cli".to_string(),
            model_hint: Some("claude-sonnet-4-20250514".to_string()),
            host: None,
            available_models: None,
        });
    }

    None
}

fn detect_openai() -> Option<DetectedCredential> {
    // 1. Check environment variable
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        if !key.trim().is_empty() {
            return Some(DetectedCredential {
                provider: "openai".to_string(),
                api_key: key.trim().to_string(),
                source: "env:OPENAI_API_KEY".to_string(),
                model_hint: Some("gpt-4o".to_string()),
                host: None,
                available_models: None,
            });
        }
    }

    None
}

fn detect_openrouter() -> Option<DetectedCredential> {
    if let Ok(key) = env::var("OPENROUTER_API_KEY") {
        if !key.trim().is_empty() {
            return Some(DetectedCredential {
                provider: "openrouter".to_string(),
                api_key: key.trim().to_string(),
                source: "env:OPENROUTER_API_KEY".to_string(),
                model_hint: Some("anthropic/claude-sonnet-4-20250514".to_string()),
                host: None,
                available_models: None,
            });
        }
    }

    None
}

fn detect_glm() -> Option<DetectedCredential> {
    // Check GLM_API_KEY first, then ZHIPUAI_API_KEY
    if let Ok(key) = env::var("GLM_API_KEY") {
        if !key.trim().is_empty() {
            return Some(DetectedCredential {
                provider: "glm".to_string(),
                api_key: key.trim().to_string(),
                source: "env:GLM_API_KEY".to_string(),
                model_hint: Some("glm-4v-flash".to_string()),
                host: None,
                available_models: None,
            });
        }
    }

    if let Ok(key) = env::var("ZHIPUAI_API_KEY") {
        if !key.trim().is_empty() {
            return Some(DetectedCredential {
                provider: "glm".to_string(),
                api_key: key.trim().to_string(),
                source: "env:ZHIPUAI_API_KEY".to_string(),
                model_hint: Some("glm-4v-flash".to_string()),
                host: None,
                available_models: None,
            });
        }
    }

    None
}

/// Known vision-capable model name prefixes for Ollama
const VISION_MODEL_PREFIXES: &[&str] = &[
    "llava",
    "bakllava",
    "llama3.2-vision",
    "llama3-vision",
    "moondream",
    "minicpm-v",
    "nanollava",
    "llava-llama3",
    "llava-phi3",
    "obsidian",
];

/// Check if a model name indicates vision capability
fn is_vision_model(model_name: &str) -> bool {
    let lower = model_name.to_lowercase();
    VISION_MODEL_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

/// Pick the best vision model from a list, preferring larger / more capable variants.
fn pick_best_vision_model(vision_models: &[String]) -> Option<String> {
    if vision_models.is_empty() {
        return None;
    }
    // Preference order: llama3.2-vision > llava > minicpm-v > moondream > others
    let preference = ["llama3.2-vision", "llava", "minicpm-v", "bakllava", "moondream"];
    for pref in &preference {
        if let Some(m) = vision_models.iter().find(|m| m.to_lowercase().starts_with(pref)) {
            return Some(m.clone());
        }
    }
    // Fall back to first vision model
    Some(vision_models[0].clone())
}

/// Try to reach an Ollama instance at the given host and return detected models.
async fn try_ollama_host(client: &Client, host: &str) -> Option<(String, Vec<String>)> {
    let url = format!("{}/api/tags", host);
    let response = client.get(&url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body: serde_json::Value = response.json().await.ok()?;
    let models: Vec<String> = body["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Some((host.to_string(), models))
}

/// Detect a running Ollama instance and list vision-capable models.
async fn detect_ollama() -> Option<DetectedCredential> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;

    // Build candidate hosts: OLLAMA_HOST env var first, then common defaults
    let mut hosts = Vec::new();
    if let Ok(custom_host) = env::var("OLLAMA_HOST") {
        let trimmed = custom_host.trim().trim_end_matches('/').to_string();
        if !trimmed.is_empty() {
            hosts.push(trimmed);
        }
    }
    hosts.push("http://localhost:11434".to_string());
    hosts.push("http://127.0.0.1:11434".to_string());

    // Deduplicate (in case OLLAMA_HOST matches a default)
    hosts.dedup();

    for host in &hosts {
        if let Some((found_host, all_models)) = try_ollama_host(&client, host).await {
            let source = if env::var("OLLAMA_HOST").is_ok()
                && host == env::var("OLLAMA_HOST").unwrap().trim().trim_end_matches('/')
            {
                "env:OLLAMA_HOST".to_string()
            } else {
                format!("running:{}", found_host)
            };

            let vision_models: Vec<String> = all_models
                .iter()
                .filter(|m| is_vision_model(m))
                .cloned()
                .collect();

            let model_hint = pick_best_vision_model(&vision_models)
                .or_else(|| all_models.first().cloned());

            return Some(DetectedCredential {
                provider: "ollama".to_string(),
                api_key: String::new(), // Ollama doesn't use API keys
                source,
                model_hint,
                host: Some(found_host),
                available_models: Some(all_models),
            });
        }
    }

    None
}

/// Attempt to read the Anthropic API key from Claude Code CLI config files.
fn read_claude_code_key() -> Option<String> {
    let home = dirs::home_dir()?;

    // Try ~/.claude.json
    let claude_json = home.join(".claude.json");
    if let Some(key) = read_key_from_json(&claude_json, "apiKey") {
        return Some(key);
    }

    // Try ~/.claude/config.json
    let claude_config = home.join(".claude").join("config.json");
    if let Some(key) = read_key_from_json(&claude_config, "apiKey") {
        return Some(key);
    }

    None
}

/// Read a string value from a JSON file by key.
fn read_key_from_json(path: &PathBuf, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let value = parsed.get(key)?.as_str()?;
    if value.trim().is_empty() {
        return None;
    }
    Some(value.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key_normal() {
        assert_eq!(mask_key("sk-ant-1234567890abcdef"), "sk-a...cdef");
    }

    #[test]
    fn test_mask_key_short() {
        assert_eq!(mask_key("12345678"), "********");
    }

    #[test]
    fn test_mask_key_empty() {
        assert_eq!(mask_key(""), "");
    }

    #[test]
    fn test_to_payload_masks_key() {
        let cred = DetectedCredential {
            provider: "anthropic".to_string(),
            api_key: "sk-ant-1234567890abcdef".to_string(),
            source: "env:ANTHROPIC_API_KEY".to_string(),
            model_hint: Some("claude-sonnet-4-20250514".to_string()),
            host: None,
            available_models: None,
        };
        let payload = cred.to_payload();
        assert_eq!(payload.key_preview, "sk-a...cdef");
        assert_eq!(payload.provider, "anthropic");
        assert!(payload.host.is_none());
        assert!(payload.available_models.is_none());
    }

    #[test]
    fn test_to_payload_with_ollama_fields() {
        let cred = DetectedCredential {
            provider: "ollama".to_string(),
            api_key: String::new(),
            source: "running:http://localhost:11434".to_string(),
            model_hint: Some("llava:latest".to_string()),
            host: Some("http://localhost:11434".to_string()),
            available_models: Some(vec![
                "llava:latest".to_string(),
                "mistral:latest".to_string(),
            ]),
        };
        let payload = cred.to_payload();
        assert_eq!(payload.provider, "ollama");
        assert_eq!(payload.key_preview, "");
        assert_eq!(payload.host.as_deref(), Some("http://localhost:11434"));
        assert_eq!(payload.available_models.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_is_vision_model() {
        assert!(is_vision_model("llava:latest"));
        assert!(is_vision_model("llava:13b"));
        assert!(is_vision_model("bakllava:latest"));
        assert!(is_vision_model("llama3.2-vision:latest"));
        assert!(is_vision_model("moondream:latest"));
        assert!(is_vision_model("minicpm-v:latest"));
        assert!(is_vision_model("nanollava:latest"));
        assert!(is_vision_model("llava-llama3:latest"));
        assert!(is_vision_model("llava-phi3:latest"));

        assert!(!is_vision_model("mistral:latest"));
        assert!(!is_vision_model("codellama:7b"));
        assert!(!is_vision_model("phi3:latest"));
        assert!(!is_vision_model("llama3:latest"));
        assert!(!is_vision_model("gemma:latest"));
    }

    #[test]
    fn test_is_vision_model_case_insensitive() {
        assert!(is_vision_model("LLaVA:latest"));
        assert!(is_vision_model("MOONDREAM:7b"));
        assert!(is_vision_model("MiniCPM-V:latest"));
    }

    #[test]
    fn test_pick_best_vision_model_preference_order() {
        let models = vec![
            "moondream:latest".to_string(),
            "llava:latest".to_string(),
            "llama3.2-vision:latest".to_string(),
        ];
        let best = pick_best_vision_model(&models);
        assert_eq!(best.as_deref(), Some("llama3.2-vision:latest"));
    }

    #[test]
    fn test_pick_best_vision_model_llava_over_moondream() {
        let models = vec![
            "moondream:latest".to_string(),
            "llava:13b".to_string(),
        ];
        let best = pick_best_vision_model(&models);
        assert_eq!(best.as_deref(), Some("llava:13b"));
    }

    #[test]
    fn test_pick_best_vision_model_fallback() {
        let models = vec!["obsidian:latest".to_string()];
        let best = pick_best_vision_model(&models);
        assert_eq!(best.as_deref(), Some("obsidian:latest"));
    }

    #[test]
    fn test_pick_best_vision_model_empty() {
        let models: Vec<String> = vec![];
        let best = pick_best_vision_model(&models);
        assert!(best.is_none());
    }

    #[test]
    fn test_vision_model_filtering() {
        let all_models = vec![
            "llava:latest".to_string(),
            "mistral:latest".to_string(),
            "codellama:7b".to_string(),
            "moondream:latest".to_string(),
            "phi3:latest".to_string(),
        ];
        let vision: Vec<String> = all_models
            .iter()
            .filter(|m| is_vision_model(m))
            .cloned()
            .collect();
        assert_eq!(vision.len(), 2);
        assert!(vision.contains(&"llava:latest".to_string()));
        assert!(vision.contains(&"moondream:latest".to_string()));
    }
}
