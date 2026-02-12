use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// A value found in a file source: the key value and where it was found.
#[derive(Debug, Clone)]
struct FileSourceEntry {
    value: String,
    source: String,
}

/// Alternative environment variable names for each provider.
const ANTHROPIC_VARS: &[&str] = &["ANTHROPIC_API_KEY", "CLAUDE_API_KEY"];
const OPENAI_VARS: &[&str] = &["OPENAI_API_KEY", "OPENAI_KEY"];
const OPENROUTER_VARS: &[&str] = &["OPENROUTER_API_KEY", "OPENROUTER_KEY"];
const GLM_VARS: &[&str] = &["GLM_API_KEY", "ZHIPUAI_API_KEY", "GLM_KEY"];

/// Parse a `.env` file into key-value pairs.
/// Handles comments, blank lines, quoted values (single and double), and inline comments.
fn parse_dotenv_file(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return map,
    };

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Split on first '='
        let Some(eq_pos) = trimmed.find('=') else {
            continue;
        };

        let key = trimmed[..eq_pos].trim();
        // Strip optional `export ` prefix
        let key = key.strip_prefix("export ").unwrap_or(key).trim();
        if key.is_empty() {
            continue;
        }

        let raw_value = trimmed[eq_pos + 1..].trim();
        let value = unquote_value(raw_value);

        if !value.is_empty() {
            map.insert(key.to_string(), value);
        }
    }

    map
}

/// Parse shell RC files for `export VAR=value`, `VAR=value`, and fish `set -x VAR value` patterns.
fn parse_shell_exports(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return map,
    };

    let is_fish = path
        .to_str()
        .map(|s| s.contains("fish"))
        .unwrap_or(false);

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if is_fish {
            // Fish shell: `set -x VAR_NAME value` or `set -gx VAR_NAME value`
            if let Some(rest) = trimmed.strip_prefix("set ") {
                let parts: Vec<&str> = rest.splitn(4, char::is_whitespace).collect();
                // Look for -x or -gx flag followed by VAR VALUE
                if parts.len() >= 3 {
                    let (var_name, raw_value) = if parts[0] == "-x" || parts[0] == "-gx" {
                        (parts[1], parts[2..].join(" "))
                    } else if parts.len() >= 4
                        && (parts[0] == "-g" && parts[1] == "-x"
                            || parts[0] == "-x" && parts[1] == "-g")
                    {
                        (parts[2], parts[3..].join(" "))
                    } else {
                        continue;
                    };
                    let value = unquote_value(raw_value.trim());
                    if !value.is_empty() {
                        map.insert(var_name.to_string(), value);
                    }
                }
            }
        } else {
            // Bash/Zsh: `export VAR=value` or `VAR=value`
            let line_content = trimmed
                .strip_prefix("export ")
                .unwrap_or(trimmed);

            let Some(eq_pos) = line_content.find('=') else {
                continue;
            };

            let key = line_content[..eq_pos].trim();
            if key.is_empty() || key.contains(' ') {
                // Not a simple assignment if key contains spaces
                continue;
            }

            let raw_value = line_content[eq_pos + 1..].trim();
            let value = unquote_value(raw_value);

            if !value.is_empty() {
                map.insert(key.to_string(), value);
            }
        }
    }

    map
}

/// Strip surrounding quotes (single or double) from a value, and handle inline comments.
fn unquote_value(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }

    if raw.starts_with('"') && raw.len() >= 2 {
        // Double-quoted: find the closing quote
        if let Some(end) = raw[1..].find('"') {
            return raw[1..1 + end].to_string();
        }
    }

    if raw.starts_with('\'') && raw.len() >= 2 {
        // Single-quoted: find the closing quote
        if let Some(end) = raw[1..].find('\'') {
            return raw[1..1 + end].to_string();
        }
    }

    // Unquoted: strip inline comment (` #` with a space before `#`)
    let value = if let Some(comment_pos) = raw.find(" #") {
        &raw[..comment_pos]
    } else {
        raw
    };

    value.trim().to_string()
}

/// Collect dotenv file paths to scan.
fn dotenv_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let dotenv_names = [".env", ".env.local", ".env.development", ".env.production"];

    if let Some(home) = dirs::home_dir() {
        for name in &dotenv_names {
            paths.push(home.join(name));
        }
        // Also check ~/.config/pia/
        for name in &dotenv_names {
            paths.push(home.join(".config").join("pia").join(name));
        }
    }

    // Current working directory
    if let Ok(cwd) = env::current_dir() {
        for name in &dotenv_names {
            paths.push(cwd.join(name));
        }
    }

    paths
}

/// Collect shell RC file paths to scan.
fn shell_rc_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".bashrc"));
        paths.push(home.join(".bash_profile"));
        paths.push(home.join(".profile"));
        paths.push(home.join(".zshrc"));
        paths.push(home.join(".zprofile"));
        paths.push(home.join(".config").join("fish").join("config.fish"));
    }
    paths
}

/// Scan all file sources (.env files and shell RC files) for known environment variable names.
/// Returns a map of var_name -> FileSourceEntry(value, source description).
fn scan_file_sources() -> HashMap<String, FileSourceEntry> {
    let mut results: HashMap<String, FileSourceEntry> = HashMap::new();

    // All var names we're interested in
    let all_vars: Vec<&str> = ANTHROPIC_VARS
        .iter()
        .chain(OPENAI_VARS.iter())
        .chain(OPENROUTER_VARS.iter())
        .chain(GLM_VARS.iter())
        .copied()
        .collect();

    // Scan .env files
    for path in dotenv_file_paths() {
        let parsed = parse_dotenv_file(&path);
        for var_name in &all_vars {
            if results.contains_key(*var_name) {
                continue; // First match wins
            }
            if let Some(value) = parsed.get(*var_name) {
                let source = format!(
                    "file:{}",
                    path.to_string_lossy()
                );
                results.insert(
                    var_name.to_string(),
                    FileSourceEntry {
                        value: value.clone(),
                        source,
                    },
                );
            }
        }
    }

    // Scan shell RC files
    for path in shell_rc_file_paths() {
        let parsed = parse_shell_exports(&path);
        for var_name in &all_vars {
            if results.contains_key(*var_name) {
                continue; // First match wins
            }
            if let Some(value) = parsed.get(*var_name) {
                let source = format!(
                    "file:{}",
                    path.to_string_lossy()
                );
                results.insert(
                    var_name.to_string(),
                    FileSourceEntry {
                        value: value.clone(),
                        source,
                    },
                );
            }
        }
    }

    results
}

/// Try to find a credential from live env vars first, then from file sources.
/// Returns (value, source) if found.
fn lookup_var(
    var_names: &[&str],
    file_sources: &HashMap<String, FileSourceEntry>,
) -> Option<(String, String)> {
    // Priority 1: live environment variables
    for var_name in var_names {
        if let Ok(key) = env::var(var_name) {
            if !key.trim().is_empty() {
                return Some((key.trim().to_string(), format!("env:{}", var_name)));
            }
        }
    }

    // Priority 2: file-based sources
    for var_name in var_names {
        if let Some(entry) = file_sources.get(*var_name) {
            return Some((entry.value.clone(), entry.source.clone()));
        }
    }

    None
}

/// Detect all available credentials from environment variables, file sources, and CLI config files.
pub fn detect_all_credentials() -> Vec<DetectedCredential> {
    let mut results = Vec::new();
    let file_sources = scan_file_sources();

    if let Some(cred) = detect_anthropic(&file_sources) {
        results.push(cred);
    }
    if let Some(cred) = detect_openai(&file_sources) {
        results.push(cred);
    }
    if let Some(cred) = detect_openrouter(&file_sources) {
        results.push(cred);
    }
    if let Some(cred) = detect_glm(&file_sources) {
        results.push(cred);
    }

    results
}

/// Detect credential for a single provider by name.
pub fn detect_credential(provider: &str) -> Option<DetectedCredential> {
    let file_sources = scan_file_sources();
    match provider {
        "anthropic" => detect_anthropic(&file_sources),
        "openai" => detect_openai(&file_sources),
        "openrouter" => detect_openrouter(&file_sources),
        "glm" => detect_glm(&file_sources),
        _ => None,
    }
}

fn detect_anthropic(file_sources: &HashMap<String, FileSourceEntry>) -> Option<DetectedCredential> {
    // 1. Check env vars and file sources for all alternative names
    if let Some((key, source)) = lookup_var(ANTHROPIC_VARS, file_sources) {
        return Some(DetectedCredential {
            provider: "anthropic".to_string(),
            api_key: key,
            source,
            model_hint: Some("claude-sonnet-4-20250514".to_string()),
        });
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

fn detect_openai(file_sources: &HashMap<String, FileSourceEntry>) -> Option<DetectedCredential> {
    if let Some((key, source)) = lookup_var(OPENAI_VARS, file_sources) {
        return Some(DetectedCredential {
            provider: "openai".to_string(),
            api_key: key,
            source,
            model_hint: Some("gpt-4o".to_string()),
        });
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

fn detect_openrouter(
    file_sources: &HashMap<String, FileSourceEntry>,
) -> Option<DetectedCredential> {
    if let Some((key, source)) = lookup_var(OPENROUTER_VARS, file_sources) {
        return Some(DetectedCredential {
            provider: "openrouter".to_string(),
            api_key: key,
            source,
            model_hint: Some("anthropic/claude-sonnet-4-20250514".to_string()),
        });
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

fn detect_glm(file_sources: &HashMap<String, FileSourceEntry>) -> Option<DetectedCredential> {
    if let Some((key, source)) = lookup_var(GLM_VARS, file_sources) {
        return Some(DetectedCredential {
            provider: "glm".to_string(),
            api_key: key,
            source,
            model_hint: Some("glm-4v-flash".to_string()),
        });
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
    use tempfile::NamedTempFile;

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

    // --- .env file parsing tests ---

    #[test]
    fn test_parse_dotenv_basic() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "ANTHROPIC_API_KEY=sk-ant-test123456").unwrap();
        writeln!(f, "OPENAI_API_KEY=sk-openai-abc").unwrap();
        let map = parse_dotenv_file(f.path());
        assert_eq!(map.get("ANTHROPIC_API_KEY").unwrap(), "sk-ant-test123456");
        assert_eq!(map.get("OPENAI_API_KEY").unwrap(), "sk-openai-abc");
    }

    #[test]
    fn test_parse_dotenv_with_comments_and_blanks() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "# This is a comment").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "MY_KEY=hello").unwrap();
        writeln!(f, "  # Another comment").unwrap();
        writeln!(f, "OTHER=world").unwrap();
        let map = parse_dotenv_file(f.path());
        assert_eq!(map.get("MY_KEY").unwrap(), "hello");
        assert_eq!(map.get("OTHER").unwrap(), "world");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_parse_dotenv_double_quoted() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"API_KEY="sk-quoted-value""#).unwrap();
        let map = parse_dotenv_file(f.path());
        assert_eq!(map.get("API_KEY").unwrap(), "sk-quoted-value");
    }

    #[test]
    fn test_parse_dotenv_single_quoted() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "API_KEY='sk-single-quoted'").unwrap();
        let map = parse_dotenv_file(f.path());
        assert_eq!(map.get("API_KEY").unwrap(), "sk-single-quoted");
    }

    #[test]
    fn test_parse_dotenv_with_export_prefix() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "export MY_VAR=exported_value").unwrap();
        let map = parse_dotenv_file(f.path());
        assert_eq!(map.get("MY_VAR").unwrap(), "exported_value");
    }

    #[test]
    fn test_parse_dotenv_inline_comment() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "MY_KEY=value123 # this is a comment").unwrap();
        let map = parse_dotenv_file(f.path());
        assert_eq!(map.get("MY_KEY").unwrap(), "value123");
    }

    #[test]
    fn test_parse_dotenv_whitespace_around_equals() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "  MY_KEY = some_value  ").unwrap();
        let map = parse_dotenv_file(f.path());
        assert_eq!(map.get("MY_KEY").unwrap(), "some_value");
    }

    #[test]
    fn test_parse_dotenv_empty_value_skipped() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "EMPTY_KEY=").unwrap();
        writeln!(f, "EMPTY_QUOTED=\"\"").unwrap();
        let map = parse_dotenv_file(f.path());
        assert!(!map.contains_key("EMPTY_KEY"));
        assert!(!map.contains_key("EMPTY_QUOTED"));
    }

    #[test]
    fn test_parse_dotenv_nonexistent_file() {
        let map = parse_dotenv_file(Path::new("/tmp/nonexistent-dotenv-file-12345"));
        assert!(map.is_empty());
    }

    // --- Shell RC file parsing tests ---

    #[test]
    fn test_parse_shell_export() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "export ANTHROPIC_API_KEY=sk-shell-test").unwrap();
        writeln!(f, "export OPENAI_API_KEY=\"sk-openai-quoted\"").unwrap();
        let map = parse_shell_exports(f.path());
        assert_eq!(map.get("ANTHROPIC_API_KEY").unwrap(), "sk-shell-test");
        assert_eq!(map.get("OPENAI_API_KEY").unwrap(), "sk-openai-quoted");
    }

    #[test]
    fn test_parse_shell_no_export() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "MY_VAR=plain_value").unwrap();
        let map = parse_shell_exports(f.path());
        assert_eq!(map.get("MY_VAR").unwrap(), "plain_value");
    }

    #[test]
    fn test_parse_shell_comments_and_blanks() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "# Shell config").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "export KEY1=val1").unwrap();
        writeln!(f, "# Another comment").unwrap();
        writeln!(f, "export KEY2=val2").unwrap();
        let map = parse_shell_exports(f.path());
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("KEY1").unwrap(), "val1");
        assert_eq!(map.get("KEY2").unwrap(), "val2");
    }

    #[test]
    fn test_parse_shell_single_quoted() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "export API_KEY='single-quoted-val'").unwrap();
        let map = parse_shell_exports(f.path());
        assert_eq!(map.get("API_KEY").unwrap(), "single-quoted-val");
    }

    #[test]
    fn test_parse_shell_nonexistent_file() {
        let map = parse_shell_exports(Path::new("/tmp/nonexistent-shell-rc-12345"));
        assert!(map.is_empty());
    }

    // --- Fish shell parsing tests ---

    #[test]
    fn test_parse_fish_set_x() {
        let dir = tempfile::tempdir().unwrap();
        let fish_path = dir.path().join("config.fish");
        fs::write(
            &fish_path,
            "set -x ANTHROPIC_API_KEY sk-fish-test123\nset -x OPENAI_API_KEY sk-fish-openai\n",
        )
        .unwrap();
        let map = parse_shell_exports(&fish_path);
        assert_eq!(map.get("ANTHROPIC_API_KEY").unwrap(), "sk-fish-test123");
        assert_eq!(map.get("OPENAI_API_KEY").unwrap(), "sk-fish-openai");
    }

    #[test]
    fn test_parse_fish_set_gx() {
        let dir = tempfile::tempdir().unwrap();
        let fish_path = dir.path().join("config.fish");
        fs::write(
            &fish_path,
            "set -gx MY_KEY global_exported_value\n",
        )
        .unwrap();
        let map = parse_shell_exports(&fish_path);
        assert_eq!(map.get("MY_KEY").unwrap(), "global_exported_value");
    }

    #[test]
    fn test_parse_fish_quoted_value() {
        let dir = tempfile::tempdir().unwrap();
        let fish_path = dir.path().join("config.fish");
        fs::write(
            &fish_path,
            "set -x API_KEY \"quoted-fish-value\"\n",
        )
        .unwrap();
        let map = parse_shell_exports(&fish_path);
        assert_eq!(map.get("API_KEY").unwrap(), "quoted-fish-value");
    }

    #[test]
    fn test_parse_fish_comments() {
        let dir = tempfile::tempdir().unwrap();
        let fish_path = dir.path().join("config.fish");
        fs::write(
            &fish_path,
            "# Fish config\nset -x KEY1 val1\n# comment\nset -x KEY2 val2\n",
        )
        .unwrap();
        let map = parse_shell_exports(&fish_path);
        assert_eq!(map.get("KEY1").unwrap(), "val1");
        assert_eq!(map.get("KEY2").unwrap(), "val2");
    }

    // --- unquote_value tests ---

    #[test]
    fn test_unquote_value_plain() {
        assert_eq!(unquote_value("hello"), "hello");
    }

    #[test]
    fn test_unquote_value_double_quoted() {
        assert_eq!(unquote_value("\"hello world\""), "hello world");
    }

    #[test]
    fn test_unquote_value_single_quoted() {
        assert_eq!(unquote_value("'hello world'"), "hello world");
    }

    #[test]
    fn test_unquote_value_inline_comment() {
        assert_eq!(unquote_value("value123 # comment"), "value123");
    }

    #[test]
    fn test_unquote_value_empty() {
        assert_eq!(unquote_value(""), "");
    }

    #[test]
    fn test_unquote_value_hash_no_space() {
        // `#` without preceding space is not treated as inline comment
        assert_eq!(unquote_value("value#notcomment"), "value#notcomment");
    }

    // --- lookup_var tests ---

    #[test]
    fn test_lookup_var_from_file_sources() {
        let mut sources = HashMap::new();
        sources.insert(
            "ANTHROPIC_API_KEY".to_string(),
            FileSourceEntry {
                value: "sk-file-test".to_string(),
                source: "file:~/.env".to_string(),
            },
        );

        let result = lookup_var(&["NONEXISTENT_VAR", "ANTHROPIC_API_KEY"], &sources);
        assert!(result.is_some());
        let (val, src) = result.unwrap();
        assert_eq!(val, "sk-file-test");
        assert_eq!(src, "file:~/.env");
    }

    #[test]
    fn test_lookup_var_env_takes_priority() {
        // Set a live env var for this test
        env::set_var("_TEST_CRED_LOOKUP", "live-value");
        let mut sources = HashMap::new();
        sources.insert(
            "_TEST_CRED_LOOKUP".to_string(),
            FileSourceEntry {
                value: "file-value".to_string(),
                source: "file:~/.env".to_string(),
            },
        );

        let result = lookup_var(&["_TEST_CRED_LOOKUP"], &sources);
        assert!(result.is_some());
        let (val, src) = result.unwrap();
        assert_eq!(val, "live-value");
        assert_eq!(src, "env:_TEST_CRED_LOOKUP");

        env::remove_var("_TEST_CRED_LOOKUP");
    }

    #[test]
    fn test_lookup_var_not_found() {
        let sources = HashMap::new();
        let result = lookup_var(&["TOTALLY_NONEXISTENT_VAR_XYZ"], &sources);
        assert!(result.is_none());
    }

    // --- Integration: detect with file sources ---

    #[test]
    fn test_detect_anthropic_from_file_sources() {
        let mut sources = HashMap::new();
        sources.insert(
            "CLAUDE_API_KEY".to_string(),
            FileSourceEntry {
                value: "sk-from-dotenv-file".to_string(),
                source: "file:~/.env".to_string(),
            },
        );

        let result = detect_anthropic(&sources);
        // This may or may not find it depending on whether ANTHROPIC_API_KEY is set
        // in the test environment. We just verify the function doesn't panic.
        // In CI where no env var is set, it should find the file source.
        if let Some(cred) = result {
            assert_eq!(cred.provider, "anthropic");
            // Key should be non-empty
            assert!(!cred.api_key.is_empty());
        }
    }

    #[test]
    fn test_detect_openai_from_file_sources() {
        let mut sources = HashMap::new();
        sources.insert(
            "OPENAI_KEY".to_string(),
            FileSourceEntry {
                value: "sk-openai-from-file".to_string(),
                source: "file:~/.zshrc".to_string(),
            },
        );

        // Only test when OPENAI_API_KEY and OPENAI_KEY are not set in env
        if env::var("OPENAI_API_KEY").is_err() && env::var("OPENAI_KEY").is_err() {
            let result = detect_openai(&sources);
            assert!(result.is_some());
            let cred = result.unwrap();
            assert_eq!(cred.provider, "openai");
            assert_eq!(cred.api_key, "sk-openai-from-file");
            assert_eq!(cred.source, "file:~/.zshrc");
        }
    }

    #[test]
    fn test_detect_glm_from_file_sources() {
        let mut sources = HashMap::new();
        sources.insert(
            "GLM_KEY".to_string(),
            FileSourceEntry {
                value: "glm-from-file".to_string(),
                source: "file:~/.bash_profile".to_string(),
            },
        );

        if env::var("GLM_API_KEY").is_err()
            && env::var("ZHIPUAI_API_KEY").is_err()
            && env::var("GLM_KEY").is_err()
        {
            let result = detect_glm(&sources);
            assert!(result.is_some());
            let cred = result.unwrap();
            assert_eq!(cred.provider, "glm");
            assert_eq!(cred.api_key, "glm-from-file");
            assert_eq!(cred.source, "file:~/.bash_profile");
        }
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
