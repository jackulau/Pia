use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub id: String,
    pub name: String,
    pub instruction: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
}

impl TaskTemplate {
    pub fn new(name: String, instruction: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            instruction,
            category: String::new(),
            is_builtin: false,
            created_at: Utc::now(),
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
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default = "default_screenshot_quality")]
    pub screenshot_quality: u8,
    #[serde(default = "default_screenshot_max_width")]
    pub screenshot_max_width: u32,
    #[serde(default)]
    pub max_tokens_per_task: Option<u64>,
    #[serde(default)]
    pub onboarding_complete: bool,
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

fn default_screenshot_quality() -> u8 {
    80
}

fn default_screenshot_max_width() -> u32 {
    1920
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
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmConfig {
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatibleConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                default_provider: "ollama".to_string(),
                max_iterations: 150,
                confirm_dangerous_actions: true,
                show_coordinate_overlay: false,
                show_visual_feedback: true,
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
                temperature: None,
                screenshot_quality: default_screenshot_quality(),
                screenshot_max_width: default_screenshot_max_width(),
                max_tokens_per_task: None,
                onboarding_complete: false,
            },
            providers: ProvidersConfig {
                ollama: Some(OllamaConfig {
                    host: "http://localhost:11434".to_string(),
                    model: "llava".to_string(),
                    temperature: None,
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
            let mut config = Config::default();
            // First launch: populate with built-in templates
            config.populate_builtin_templates();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&path)?;
        let mut config: Config = toml::from_str(&content)?;
        // Ensure built-in templates are present (handles app updates adding new ones)
        config.populate_builtin_templates();
        Ok(config)
    }

    /// Adds any missing built-in templates without duplicating existing ones.
    /// Checks by stable ID to avoid re-adding templates the user already has.
    pub fn populate_builtin_templates(&mut self) {
        use super::builtin_templates::get_builtin_templates;

        let existing_ids: std::collections::HashSet<String> =
            self.templates.iter().map(|t| t.id.clone()).collect();

        let builtins = get_builtin_templates();
        for builtin in builtins {
            if !existing_ids.contains(&builtin.id) {
                self.templates.push(builtin);
            }
        }
    }

    /// Restores any missing built-in templates without affecting user-created ones.
    /// Returns the number of templates restored.
    pub fn restore_builtin_templates(&mut self) -> usize {
        use super::builtin_templates::get_builtin_templates;

        let existing_ids: std::collections::HashSet<String> =
            self.templates.iter().map(|t| t.id.clone()).collect();

        let builtins = get_builtin_templates();
        let mut restored = 0;
        for builtin in builtins {
            if !existing_ids.contains(&builtin.id) {
                self.templates.push(builtin);
                restored += 1;
            }
        }
        restored
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
                        temperature: None,
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
                        temperature: None,
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
                        temperature: None,
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
                        temperature: None,
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
    fn test_task_template_new_has_defaults() {
        let template = TaskTemplate::new("Test".to_string(), "Do something".to_string());
        assert!(!template.is_builtin);
        assert!(template.category.is_empty());
        assert!(!template.id.starts_with("builtin-"));
    }

    #[test]
    fn test_populate_builtin_templates_on_empty_config() {
        let mut config = Config::default();
        assert!(config.templates.is_empty());

        config.populate_builtin_templates();

        assert!(
            config.templates.len() >= 12,
            "Expected at least 12 templates, got {}",
            config.templates.len()
        );
        // All should be built-in
        assert!(config.templates.iter().all(|t| t.is_builtin));
    }

    #[test]
    fn test_populate_builtin_templates_does_not_duplicate() {
        let mut config = Config::default();

        // Populate once
        config.populate_builtin_templates();
        let count_after_first = config.templates.len();

        // Populate again - should not add duplicates
        config.populate_builtin_templates();
        assert_eq!(
            config.templates.len(),
            count_after_first,
            "Second populate should not add duplicates"
        );
    }

    #[test]
    fn test_populate_preserves_user_templates() {
        let mut config = Config::default();

        // Add a user template
        let user_template = TaskTemplate::new("My Template".to_string(), "Do X".to_string());
        config.templates.push(user_template.clone());

        // Populate built-in templates
        config.populate_builtin_templates();

        // User template should still be there
        assert!(config.templates.iter().any(|t| t.id == user_template.id));
        // Built-in templates should also be there
        assert!(config.templates.iter().any(|t| t.is_builtin));
    }

    #[test]
    fn test_restore_builtin_templates_counts_correctly() {
        let mut config = Config::default();
        config.populate_builtin_templates();
        let initial_count = config.templates.len();

        // Remove one built-in template
        config.templates.retain(|t| t.id != "builtin-fill-web-form");
        assert_eq!(config.templates.len(), initial_count - 1);

        // Restore should return 1
        let restored = config.restore_builtin_templates();
        assert_eq!(restored, 1);
        assert_eq!(config.templates.len(), initial_count);

        // Restore again should return 0
        let restored_again = config.restore_builtin_templates();
        assert_eq!(restored_again, 0);
    }

    #[test]
    fn test_builtin_templates_serialize_deserialize() {
        let mut config = Config::default();
        config.populate_builtin_templates();

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");

        // Deserialize back
        let loaded: Config = toml::from_str(&toml_str).expect("Failed to deserialize config");

        assert_eq!(loaded.templates.len(), config.templates.len());

        // Verify is_builtin and category survive the round-trip
        for (original, loaded) in config.templates.iter().zip(loaded.templates.iter()) {
            assert_eq!(original.id, loaded.id);
            assert_eq!(original.is_builtin, loaded.is_builtin);
            assert_eq!(original.category, loaded.category);
            assert_eq!(original.name, loaded.name);
            assert_eq!(original.instruction, loaded.instruction);
        }
    }

    #[test]
    fn test_backward_compat_deserialize_without_new_fields() {
        // Simulate old config TOML that doesn't have is_builtin or category
        let old_toml = r#"
[general]
default_provider = "ollama"
max_iterations = 150
confirm_dangerous_actions = true

[providers]

[[templates]]
id = "old-template-123"
name = "Old Template"
instruction = "Do old thing"
created_at = "2024-01-01T00:00:00Z"
"#;

        let config: Config = toml::from_str(old_toml).expect("Failed to parse old config format");
        assert_eq!(config.templates.len(), 1);
        assert_eq!(config.templates[0].name, "Old Template");
        assert!(!config.templates[0].is_builtin); // default false
        assert!(config.templates[0].category.is_empty()); // default empty
    }
}
