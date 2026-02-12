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
    pub is_valid: bool,
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
        let is_valid = validate_key_format(&self.provider, &self.api_key);
        DetectedCredentialPayload {
            provider: self.provider.clone(),
            source: self.source.clone(),
            model_hint: self.model_hint.clone(),
            key_preview,
            is_valid,
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

/// Common placeholder patterns that indicate a key is not real.
const PLACEHOLDER_PATTERNS: &[&str] = &[
    "your-key-here",
    "your-api-key",
    "your_key_here",
    "your_api_key",
    "sk-xxx",
    "sk-your",
    "todo",
    "placeholder",
    "insert-key",
    "insert_key",
    "replace-me",
    "replace_me",
    "changeme",
    "change-me",
    "test-key",
    "test_key",
    "fake-key",
    "fake_key",
    "example",
    "dummy",
];

/// Validate that an API key matches expected format for the given provider.
/// Returns `true` if the key appears to be a genuine, well-formed key.
/// Invalid keys are still returned by detection — they are just flagged.
pub fn validate_key_format(provider: &str, key: &str) -> bool {
    let trimmed = key.trim();

    // Reject keys shorter than 10 characters
    if trimmed.len() < 10 {
        return false;
    }

    // Reject common placeholder patterns (case-insensitive)
    let lower = trimmed.to_lowercase();
    for pattern in PLACEHOLDER_PATTERNS {
        if lower.contains(pattern) {
            return false;
        }
    }

    // Reject keys made of only repeated characters (e.g. "aaaaaaaaaa")
    if trimmed.len() > 1 {
        let first = trimmed.chars().next().unwrap();
        if trimmed.chars().all(|c| c == first) {
            return false;
        }
    }

    // Provider-specific prefix validation
    match provider {
        "anthropic" => trimmed.starts_with("sk-ant-"),
        "openai" => {
            trimmed.starts_with("sk-proj-")
                || (trimmed.starts_with("sk-")
                    && !trimmed.starts_with("sk-ant-")
                    && !trimmed.starts_with("sk-or-"))
        }
        "openrouter" => trimmed.starts_with("sk-or-"),
        "glm" => {
            // GLM/Zhipu keys use JWT-like format with a '.' separator
            trimmed.contains('.')
        }
        _ => true, // Unknown providers: pass if basic checks passed
    }
}

/// Detect which provider an API key likely belongs to based on its prefix.
pub fn detect_provider_from_key(key: &str) -> Option<&str> {
    let trimmed = key.trim();
    if trimmed.starts_with("sk-ant-") {
        Some("anthropic")
    } else if trimmed.starts_with("sk-or-") {
        Some("openrouter")
    } else if trimmed.starts_with("sk-proj-") {
        Some("openai")
    } else if trimmed.starts_with("sk-") {
        // Generic sk- prefix (not ant or or) — most likely OpenAI
        Some("openai")
    } else if trimmed.contains('.') && trimmed.len() > 20 {
        // JWT-like format with dot separator — likely GLM/Zhipu
        Some("glm")
    } else {
        None
    }
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
        assert!(payload.is_valid);
    }

    #[test]
    fn test_to_payload_invalid_key() {
        let cred = DetectedCredential {
            provider: "anthropic".to_string(),
            api_key: "sk-xxx-placeholder".to_string(),
            source: "env:ANTHROPIC_API_KEY".to_string(),
            model_hint: None,
        };
        let payload = cred.to_payload();
        assert!(!payload.is_valid);
    }

    // --- validate_key_format tests ---

    #[test]
    fn test_validate_anthropic_valid() {
        assert!(validate_key_format("anthropic", "sk-ant-abcdef1234567890"));
    }

    #[test]
    fn test_validate_anthropic_wrong_prefix() {
        assert!(!validate_key_format("anthropic", "sk-proj-abcdef1234567890"));
        assert!(!validate_key_format("anthropic", "sk-or-abcdef1234567890"));
        assert!(!validate_key_format("anthropic", "sk-abcdef1234567890"));
    }

    #[test]
    fn test_validate_openai_valid() {
        assert!(validate_key_format("openai", "sk-abcdef1234567890"));
        assert!(validate_key_format("openai", "sk-proj-abcdef1234567890"));
    }

    #[test]
    fn test_validate_openai_wrong_prefix() {
        assert!(!validate_key_format("openai", "sk-ant-abcdef1234567890"));
        assert!(!validate_key_format("openai", "sk-or-abcdef1234567890"));
    }

    #[test]
    fn test_validate_openrouter_valid() {
        assert!(validate_key_format("openrouter", "sk-or-abcdef1234567890"));
    }

    #[test]
    fn test_validate_openrouter_wrong_prefix() {
        assert!(!validate_key_format("openrouter", "sk-ant-abcdef1234567890"));
        assert!(!validate_key_format("openrouter", "sk-abcdef1234567890"));
    }

    #[test]
    fn test_validate_glm_valid() {
        assert!(validate_key_format("glm", "abc123def.ghijklmnop456"));
    }

    #[test]
    fn test_validate_glm_missing_dot() {
        assert!(!validate_key_format("glm", "abc123defghijklmnop456"));
    }

    #[test]
    fn test_validate_short_key_rejected() {
        assert!(!validate_key_format("anthropic", "sk-ant-a"));
        assert!(!validate_key_format("openai", "sk-short"));
        assert!(!validate_key_format("openrouter", "sk-or-x"));
    }

    #[test]
    fn test_validate_placeholder_rejected() {
        assert!(!validate_key_format("anthropic", "sk-ant-your-key-here-abcdef"));
        assert!(!validate_key_format("openai", "sk-TODO-replace-this-later"));
        assert!(!validate_key_format("openai", "sk-placeholder-key-value"));
        assert!(!validate_key_format("anthropic", "sk-ant-PLACEHOLDER1234"));
        assert!(!validate_key_format("openai", "sk-example-key-1234567"));
        assert!(!validate_key_format("openai", "sk-dummy-key-123456789"));
        assert!(!validate_key_format("openrouter", "sk-or-changeme-please12"));
    }

    #[test]
    fn test_validate_repeated_chars_rejected() {
        assert!(!validate_key_format("anthropic", "aaaaaaaaaa"));
        assert!(!validate_key_format("openai", "xxxxxxxxxxxx"));
    }

    #[test]
    fn test_validate_unknown_provider_passes_basic() {
        // Unknown providers pass as long as basic checks pass
        assert!(validate_key_format("some-custom", "a-valid-looking-key-1234"));
    }

    #[test]
    fn test_validate_unknown_provider_still_rejects_short() {
        assert!(!validate_key_format("some-custom", "short"));
    }

    // --- detect_provider_from_key tests ---

    #[test]
    fn test_detect_provider_anthropic() {
        assert_eq!(detect_provider_from_key("sk-ant-abcdef1234"), Some("anthropic"));
    }

    #[test]
    fn test_detect_provider_openai_sk() {
        assert_eq!(detect_provider_from_key("sk-abcdef1234567890"), Some("openai"));
    }

    #[test]
    fn test_detect_provider_openai_proj() {
        assert_eq!(detect_provider_from_key("sk-proj-abcdef123456"), Some("openai"));
    }

    #[test]
    fn test_detect_provider_openrouter() {
        assert_eq!(detect_provider_from_key("sk-or-abcdef1234567890"), Some("openrouter"));
    }

    #[test]
    fn test_detect_provider_glm() {
        assert_eq!(detect_provider_from_key("abc123def456789.ghijklmnop0123456"), Some("glm"));
    }

    #[test]
    fn test_detect_provider_unknown() {
        assert_eq!(detect_provider_from_key("xyz-unknown-key"), None);
    }

    #[test]
    fn test_detect_provider_empty() {
        assert_eq!(detect_provider_from_key(""), None);
    }

    #[test]
    fn test_detect_provider_short_dot_not_glm() {
        // Short key with dot should not be detected as GLM
        assert_eq!(detect_provider_from_key("a.b"), None);
    }

    #[test]
    fn test_validate_whitespace_trimmed() {
        assert!(validate_key_format("anthropic", "  sk-ant-abcdef1234567890  "));
    }
}
