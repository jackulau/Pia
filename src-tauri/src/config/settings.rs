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
    pub created_at: DateTime<Utc>,
}

impl TaskTemplate {
    pub fn new(name: String, instruction: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            instruction,
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
