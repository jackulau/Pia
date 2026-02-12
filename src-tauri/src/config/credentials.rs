use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

    // 3. Check CLI tool config files (aider, litellm, etc.)
    if let Some((key, source)) = scan_config_files("anthropic").into_iter().next() {
        return Some(DetectedCredential {
            provider: "anthropic".to_string(),
            api_key: key,
            source,
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

    // 2. Check CLI tool config files (openai auth, copilot, aider)
    if let Some((key, source)) = scan_config_files("openai").into_iter().next() {
        return Some(DetectedCredential {
            provider: "openai".to_string(),
            api_key: key,
            source,
            model_hint: Some("gpt-4o".to_string()),
        });
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

    // 2. Check CLI tool config files (aider)
    if let Some((key, source)) = scan_config_files("openrouter").into_iter().next() {
        return Some(DetectedCredential {
            provider: "openrouter".to_string(),
            api_key: key,
            source,
            model_hint: Some("anthropic/claude-sonnet-4-20250514".to_string()),
        });
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

/// Read a string value from a JSON file by top-level key.
fn read_key_from_json(path: &PathBuf, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let value = parsed.get(key)?.as_str()?;
    if value.trim().is_empty() {
        return None;
    }
    Some(value.trim().to_string())
}

/// Read a string value from a YAML file by top-level key.
/// Uses simple line-based parsing to avoid adding a serde_yaml dependency.
fn read_key_from_yaml(path: &Path, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let prefix = format!("{}:", key);
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&prefix) {
            let value_part = trimmed[prefix.len()..].trim();
            // Strip surrounding quotes if present
            let unquoted = value_part
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| {
                    value_part
                        .strip_prefix('\'')
                        .and_then(|s| s.strip_suffix('\''))
                })
                .unwrap_or(value_part);
            if !unquoted.is_empty() {
                return Some(unquoted.to_string());
            }
        }
    }
    None
}

/// Read an OAuth token from GitHub Copilot hosts.json.
/// The file maps host URLs to objects containing `oauth_token`.
fn read_copilot_oauth_token(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let obj = parsed.as_object()?;
    // Iterate host entries and return the first oauth_token found
    for (_host, entry) in obj {
        if let Some(token) = entry.get("oauth_token").and_then(|v| v.as_str()) {
            let trimmed = token.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Scan well-known config files for a given provider.
/// Returns a list of (api_key, source_description) tuples.
fn scan_config_files(provider: &str) -> Vec<(String, String)> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let mut results = Vec::new();

    match provider {
        "anthropic" => {
            // Aider config: ~/.config/aider/.aider.conf.yml or ~/.aider.conf.yml
            let aider_paths = [
                home.join(".config/aider/.aider.conf.yml"),
                home.join(".aider.conf.yml"),
            ];
            for path in &aider_paths {
                if let Some(key) = read_key_from_yaml(path, "anthropic-api-key") {
                    results.push((key, format!("config:{}", path.display())));
                    break; // Only take first match from aider
                }
            }

            // LiteLLM config: ~/.config/litellm/config.yaml
            let litellm_path = home.join(".config/litellm/config.yaml");
            if let Some(key) = read_key_from_yaml(&litellm_path, "api_key") {
                // Only include if it looks like an Anthropic key
                if key.starts_with("sk-ant-") {
                    results.push((key, format!("config:{}", litellm_path.display())));
                }
            }
        }
        "openai" => {
            // OpenAI CLI auth: ~/.config/openai/auth.json
            let openai_auth = home.join(".config/openai/auth.json");
            if let Some(key) = read_key_from_json(&openai_auth, "api_key") {
                results.push((key, format!("config:{}", openai_auth.display())));
            }

            // GitHub Copilot hosts.json
            let copilot_paths = [
                home.join(".config/github-copilot/hosts.json"),
                home.join(".config/github-copilot/apps.json"),
            ];
            for path in &copilot_paths {
                if let Some(token) = read_copilot_oauth_token(path) {
                    results.push((token, format!("config:{}", path.display())));
                    break;
                }
            }

            // Aider config: openai-api-key
            let aider_paths = [
                home.join(".config/aider/.aider.conf.yml"),
                home.join(".aider.conf.yml"),
            ];
            for path in &aider_paths {
                if let Some(key) = read_key_from_yaml(path, "openai-api-key") {
                    results.push((key, format!("config:{}", path.display())));
                    break;
                }
            }
        }
        "openrouter" => {
            // Aider config: openrouter-api-key
            let aider_paths = [
                home.join(".config/aider/.aider.conf.yml"),
                home.join(".aider.conf.yml"),
            ];
            for path in &aider_paths {
                if let Some(key) = read_key_from_yaml(path, "openrouter-api-key") {
                    results.push((key, format!("config:{}", path.display())));
                    break;
                }
            }
        }
        _ => {}
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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

    // --- YAML parsing tests ---

    #[test]
    fn test_read_key_from_yaml_simple() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "anthropic-api-key: sk-ant-test123456789abc").unwrap();
        writeln!(tmp, "openai-api-key: sk-openai-xyz").unwrap();
        tmp.flush().unwrap();

        let path = tmp.path();
        assert_eq!(
            read_key_from_yaml(path, "anthropic-api-key"),
            Some("sk-ant-test123456789abc".to_string())
        );
        assert_eq!(
            read_key_from_yaml(path, "openai-api-key"),
            Some("sk-openai-xyz".to_string())
        );
        assert_eq!(read_key_from_yaml(path, "nonexistent-key"), None);
    }

    #[test]
    fn test_read_key_from_yaml_quoted_values() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "double-quoted: \"sk-double-quoted-key\"").unwrap();
        writeln!(tmp, "single-quoted: 'sk-single-quoted-key'").unwrap();
        tmp.flush().unwrap();

        let path = tmp.path();
        assert_eq!(
            read_key_from_yaml(path, "double-quoted"),
            Some("sk-double-quoted-key".to_string())
        );
        assert_eq!(
            read_key_from_yaml(path, "single-quoted"),
            Some("sk-single-quoted-key".to_string())
        );
    }

    #[test]
    fn test_read_key_from_yaml_empty_value() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "empty-key:").unwrap();
        writeln!(tmp, "blank-key:   ").unwrap();
        writeln!(tmp, "quoted-empty: \"\"").unwrap();
        tmp.flush().unwrap();

        let path = tmp.path();
        assert_eq!(read_key_from_yaml(path, "empty-key"), None);
        assert_eq!(read_key_from_yaml(path, "blank-key"), None);
        assert_eq!(read_key_from_yaml(path, "quoted-empty"), None);
    }

    #[test]
    fn test_read_key_from_yaml_missing_file() {
        let path = Path::new("/nonexistent/path/.aider.conf.yml");
        assert_eq!(read_key_from_yaml(path, "anthropic-api-key"), None);
    }

    #[test]
    fn test_read_key_from_yaml_with_comments() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "# This is a comment").unwrap();
        writeln!(tmp, "anthropic-api-key: sk-ant-real-key").unwrap();
        writeln!(tmp, "# openai-api-key: sk-fake-key").unwrap();
        tmp.flush().unwrap();

        let path = tmp.path();
        assert_eq!(
            read_key_from_yaml(path, "anthropic-api-key"),
            Some("sk-ant-real-key".to_string())
        );
        // The commented-out line should not match since it starts with #
        assert_eq!(read_key_from_yaml(path, "openai-api-key"), None);
    }

    // --- JSON config file tests ---

    #[test]
    fn test_read_key_from_json_openai_auth() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, r#"{{"api_key": "sk-openai-testkey123"}}"#).unwrap();
        tmp.flush().unwrap();

        let path = tmp.path().to_path_buf();
        assert_eq!(
            read_key_from_json(&path, "api_key"),
            Some("sk-openai-testkey123".to_string())
        );
    }

    #[test]
    fn test_read_key_from_json_missing_key() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, r#"{{"other_field": "value"}}"#).unwrap();
        tmp.flush().unwrap();

        let path = tmp.path().to_path_buf();
        assert_eq!(read_key_from_json(&path, "api_key"), None);
    }

    #[test]
    fn test_read_key_from_json_empty_value() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, r#"{{"api_key": ""}}"#).unwrap();
        tmp.flush().unwrap();

        let path = tmp.path().to_path_buf();
        assert_eq!(read_key_from_json(&path, "api_key"), None);
    }

    #[test]
    fn test_read_key_from_json_missing_file() {
        let path = PathBuf::from("/nonexistent/path/auth.json");
        assert_eq!(read_key_from_json(&path, "api_key"), None);
    }

    #[test]
    fn test_read_key_from_json_invalid_json() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "not valid json {{{{").unwrap();
        tmp.flush().unwrap();

        let path = tmp.path().to_path_buf();
        assert_eq!(read_key_from_json(&path, "api_key"), None);
    }

    // --- Copilot hosts.json tests ---

    #[test]
    fn test_read_copilot_oauth_token() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{"github.com": {{"user": "test", "oauth_token": "gho_copilot_token_abc"}}}}"#
        )
        .unwrap();
        tmp.flush().unwrap();

        assert_eq!(
            read_copilot_oauth_token(tmp.path()),
            Some("gho_copilot_token_abc".to_string())
        );
    }

    #[test]
    fn test_read_copilot_oauth_token_empty() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{"github.com": {{"user": "test", "oauth_token": ""}}}}"#
        )
        .unwrap();
        tmp.flush().unwrap();

        assert_eq!(read_copilot_oauth_token(tmp.path()), None);
    }

    #[test]
    fn test_read_copilot_oauth_token_missing_file() {
        let path = Path::new("/nonexistent/hosts.json");
        assert_eq!(read_copilot_oauth_token(path), None);
    }

    #[test]
    fn test_read_copilot_oauth_token_no_token_field() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, r#"{{"github.com": {{"user": "test"}}}}"#).unwrap();
        tmp.flush().unwrap();

        assert_eq!(read_copilot_oauth_token(tmp.path()), None);
    }

    // --- Aider full YAML config test ---

    #[test]
    fn test_aider_config_all_providers() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "# Aider configuration").unwrap();
        writeln!(tmp, "model: claude-3-opus-20240229").unwrap();
        writeln!(tmp, "anthropic-api-key: sk-ant-aider-key").unwrap();
        writeln!(tmp, "openai-api-key: sk-openai-aider-key").unwrap();
        writeln!(tmp, "openrouter-api-key: sk-or-v1-aider-key").unwrap();
        writeln!(tmp, "auto-commits: false").unwrap();
        tmp.flush().unwrap();

        let path = tmp.path();
        assert_eq!(
            read_key_from_yaml(path, "anthropic-api-key"),
            Some("sk-ant-aider-key".to_string())
        );
        assert_eq!(
            read_key_from_yaml(path, "openai-api-key"),
            Some("sk-openai-aider-key".to_string())
        );
        assert_eq!(
            read_key_from_yaml(path, "openrouter-api-key"),
            Some("sk-or-v1-aider-key".to_string())
        );
        // Should not match non-key fields
        assert_eq!(read_key_from_yaml(path, "nonexistent"), None);
    }
}
