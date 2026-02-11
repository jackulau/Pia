use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Metadata about a detected credential (safe to send to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedCredentialPayload {
    pub provider: String,
    pub source: String,
    pub model_hint: Option<String>,
    pub key_preview: String,
}

/// Internal struct with full API key (never sent to frontend)
#[derive(Debug, Clone)]
pub struct DetectedCredential {
    pub provider: String,
    pub api_key: String,
    pub source: String,
    pub model_hint: Option<String>,
}

impl DetectedCredential {
    pub fn to_payload(&self) -> DetectedCredentialPayload {
        let key_preview = mask_key(&self.api_key);
        DetectedCredentialPayload {
            provider: self.provider.clone(),
            source: self.source.clone(),
            model_hint: self.model_hint.clone(),
            key_preview,
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

/// Detect all available credentials from environment variables and CLI config files.
pub fn detect_all_credentials() -> Vec<DetectedCredential> {
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

    results
}

/// Detect credential for a single provider by name.
pub fn detect_credential(provider: &str) -> Option<DetectedCredential> {
    match provider {
        "anthropic" => detect_anthropic(),
        "openai" => detect_openai(),
        "openrouter" => detect_openrouter(),
        "glm" => detect_glm(),
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
        };
        let payload = cred.to_payload();
        assert_eq!(payload.key_preview, "sk-a...cdef");
        assert_eq!(payload.provider, "anthropic");
    }
}
