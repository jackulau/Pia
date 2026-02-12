use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),
    #[error("Config directory not found")]
    NoDirFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateVariable {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}

/// Extract `{{variable_name}}` placeholders from instruction text.
/// Returns unique variables in order of first appearance.
pub fn extract_variables(instruction: &str) -> Vec<TemplateVariable> {
    let re = Regex::new(r"\{\{([A-Za-z_][A-Za-z0-9_]*)\}\}").unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut vars = Vec::new();
    for cap in re.captures_iter(instruction) {
        let name = cap[1].to_string();
        if seen.insert(name.clone()) {
            vars.push(TemplateVariable {
                name,
                description: None,
                default_value: None,
            });
        }
    }
    vars
}

/// Replace `{{variable_name}}` placeholders with provided values.
/// Variables without a provided value are left as-is.
pub fn render_instruction(instruction: &str, values: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\{\{([A-Za-z_][A-Za-z0-9_]*)\}\}").unwrap();
    re.replace_all(instruction, |caps: &regex::Captures| {
        let name = &caps[1];
        values.get(name).cloned().unwrap_or_else(|| caps[0].to_string())
    })
    .to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub id: String,
    pub name: String,
    pub instruction: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub variables: Vec<TemplateVariable>,
}

impl TaskTemplate {
    pub fn new(name: String, instruction: String) -> Self {
        let variables = extract_variables(&instruction);
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            instruction,
            created_at: Utc::now(),
            variables,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub templates: Vec<TaskTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub default_provider: String,
    pub max_iterations: u32,
    pub confirm_dangerous_actions: bool,
    #[serde(default)]
    pub show_coordinate_overlay: bool,
    #[serde(default = "default_show_visual_feedback")]
    pub show_visual_feedback: bool,
    #[serde(default = "default_global_hotkey")]
    pub global_hotkey: Option<String>,
    #[serde(default = "default_queue_failure_mode")]
    pub queue_failure_mode: String,
    #[serde(default = "default_queue_delay_ms")]
    pub queue_delay_ms: u32,
    #[serde(default)]
    pub preview_mode: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u32,
    #[serde(default = "default_enable_self_correction")]
    pub enable_self_correction: bool,
    #[serde(default = "default_speed_multiplier")]
    pub speed_multiplier: f32,
    #[serde(default = "default_true")]
    pub voice_input_enabled: bool,
    #[serde(default)]
    pub voice_auto_submit: bool,
    #[serde(default = "default_voice_language")]
    pub voice_language: String,
    #[serde(default = "default_connect_timeout_secs")]
    pub connect_timeout_secs: u64,
    #[serde(default = "default_response_timeout_secs")]
    pub response_timeout_secs: u64,
}

fn default_global_hotkey() -> Option<String> {
    Some("CmdOrCtrl+Shift+P".to_string())
}

fn default_queue_failure_mode() -> String {
    "stop".to_string()
}

fn default_queue_delay_ms() -> u32 {
    500
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay_ms() -> u32 {
    1000
}

fn default_enable_self_correction() -> bool {
    true
}

fn default_speed_multiplier() -> f32 {
    1.0
}

fn default_show_visual_feedback() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_voice_language() -> String {
    "en-US".to_string()
}

fn default_connect_timeout_secs() -> u64 {
    30
}

fn default_response_timeout_secs() -> u64 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,
    #[serde(default)]
    pub anthropic: Option<AnthropicConfig>,
    #[serde(default)]
    pub openai: Option<OpenAIConfig>,
    #[serde(default)]
    pub openrouter: Option<OpenRouterConfig>,
    #[serde(default)]
    pub glm: Option<GlmConfig>,
    #[serde(default)]
    pub openai_compatible: Option<OpenAICompatibleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub host: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatibleConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    pub model: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                default_provider: "ollama".to_string(),
                max_iterations: 150,
                confirm_dangerous_actions: true,
                show_coordinate_overlay: false,
                show_visual_feedback: false,
                global_hotkey: default_global_hotkey(),
                queue_failure_mode: "stop".to_string(),
                queue_delay_ms: 500,
                preview_mode: false,
                max_retries: default_max_retries(),
                retry_delay_ms: default_retry_delay_ms(),
                enable_self_correction: default_enable_self_correction(),
                speed_multiplier: 1.0,
                voice_input_enabled: true,
                voice_auto_submit: false,
                voice_language: "en-US".to_string(),
                connect_timeout_secs: default_connect_timeout_secs(),
                response_timeout_secs: default_response_timeout_secs(),
            },
            providers: ProvidersConfig {
                ollama: Some(OllamaConfig {
                    host: "http://localhost:11434".to_string(),
                    model: "llava".to_string(),
                }),
                anthropic: None,
                openai: None,
                openrouter: None,
                glm: None,
                openai_compatible: None,
            },
            templates: Vec::new(),
        }
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir().ok_or(ConfigError::NoDirFound)?;
        Ok(config_dir.join("pia").join("config.toml"))
    }

    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;

        if !path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn update_provider_api_key(&mut self, provider: &str, api_key: &str) {
        match provider {
            "anthropic" => {
                if let Some(ref mut config) = self.providers.anthropic {
                    config.api_key = api_key.to_string();
                } else {
                    self.providers.anthropic = Some(AnthropicConfig {
                        api_key: api_key.to_string(),
                        model: "claude-sonnet-4-20250514".to_string(),
                    });
                }
            }
            "openai" => {
                if let Some(ref mut config) = self.providers.openai {
                    config.api_key = api_key.to_string();
                } else {
                    self.providers.openai = Some(OpenAIConfig {
                        api_key: api_key.to_string(),
                        model: "gpt-4o".to_string(),
                    });
                }
            }
            "openrouter" => {
                if let Some(ref mut config) = self.providers.openrouter {
                    config.api_key = api_key.to_string();
                } else {
                    self.providers.openrouter = Some(OpenRouterConfig {
                        api_key: api_key.to_string(),
                        model: "anthropic/claude-sonnet-4-20250514".to_string(),
                    });
                }
            }
            "glm" => {
                if let Some(ref mut config) = self.providers.glm {
                    config.api_key = api_key.to_string();
                } else {
                    self.providers.glm = Some(GlmConfig {
                        api_key: api_key.to_string(),
                        model: "glm-4v".to_string(),
                    });
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_variables_single() {
        let vars = extract_variables("Go to {{url}}");
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "url");
    }

    #[test]
    fn test_extract_variables_multiple() {
        let vars = extract_variables("Fill form at {{url}} with email {{email}} and name {{name}}");
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0].name, "url");
        assert_eq!(vars[1].name, "email");
        assert_eq!(vars[2].name, "name");
    }

    #[test]
    fn test_extract_variables_dedup() {
        let vars = extract_variables("{{url}} and again {{url}}");
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "url");
    }

    #[test]
    fn test_extract_variables_none() {
        let vars = extract_variables("No variables here");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_extract_variables_empty_braces() {
        let vars = extract_variables("Empty {{}} braces");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_extract_variables_nested_braces() {
        let vars = extract_variables("{{{name}}}");
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "name");
    }

    #[test]
    fn test_extract_variables_underscores() {
        let vars = extract_variables("{{my_var}} and {{another_one}}");
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].name, "my_var");
        assert_eq!(vars[1].name, "another_one");
    }

    #[test]
    fn test_extract_variables_with_numbers() {
        let vars = extract_variables("{{field1}} and {{item2}}");
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].name, "field1");
        assert_eq!(vars[1].name, "item2");
    }

    #[test]
    fn test_extract_variables_ignores_spaces() {
        // Spaces inside braces should not match
        let vars = extract_variables("{{ name }} is not valid");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_render_instruction_all_values() {
        let mut values = HashMap::new();
        values.insert("url".to_string(), "https://example.com".to_string());
        values.insert("email".to_string(), "test@test.com".to_string());
        let result = render_instruction("Go to {{url}} with {{email}}", &values);
        assert_eq!(result, "Go to https://example.com with test@test.com");
    }

    #[test]
    fn test_render_instruction_missing_values() {
        let values = HashMap::new();
        let result = render_instruction("Go to {{url}}", &values);
        assert_eq!(result, "Go to {{url}}");
    }

    #[test]
    fn test_render_instruction_partial_values() {
        let mut values = HashMap::new();
        values.insert("url".to_string(), "https://example.com".to_string());
        let result = render_instruction("Go to {{url}} with {{email}}", &values);
        assert_eq!(result, "Go to https://example.com with {{email}}");
    }

    #[test]
    fn test_render_instruction_extra_values() {
        let mut values = HashMap::new();
        values.insert("url".to_string(), "https://example.com".to_string());
        values.insert("unused".to_string(), "ignored".to_string());
        let result = render_instruction("Go to {{url}}", &values);
        assert_eq!(result, "Go to https://example.com");
    }

    #[test]
    fn test_render_instruction_no_variables() {
        let values = HashMap::new();
        let result = render_instruction("No variables here", &values);
        assert_eq!(result, "No variables here");
    }

    #[test]
    fn test_render_instruction_duplicate_variables() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());
        let result = render_instruction("Hello {{name}}, your name is {{name}}", &values);
        assert_eq!(result, "Hello Alice, your name is Alice");
    }

    #[test]
    fn test_task_template_new_extracts_variables() {
        let template = TaskTemplate::new(
            "Test".to_string(),
            "Go to {{url}} with {{email}}".to_string(),
        );
        assert_eq!(template.variables.len(), 2);
        assert_eq!(template.variables[0].name, "url");
        assert_eq!(template.variables[1].name, "email");
    }

    #[test]
    fn test_task_template_new_no_variables() {
        let template = TaskTemplate::new(
            "Test".to_string(),
            "Simple instruction".to_string(),
        );
        assert!(template.variables.is_empty());
    }

    #[test]
    fn test_backwards_compatibility_deserialize() {
        // Simulate a template without the variables field (old format)
        let toml_str = r#"
            id = "abc-123"
            name = "Old Template"
            instruction = "Go to {{url}}"
            created_at = "2024-01-01T00:00:00Z"
        "#;
        let template: TaskTemplate = toml::from_str(toml_str).unwrap();
        assert_eq!(template.name, "Old Template");
        // variables field defaults to empty vec via serde(default)
        assert!(template.variables.is_empty());
    }
}
