use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

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
pub struct Config {
    pub general: GeneralConfig,
    pub providers: ProvidersConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub default_provider: String,
    pub max_iterations: u32,
    pub confirm_dangerous_actions: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u32,
    #[serde(default = "default_enable_self_correction")]
    pub enable_self_correction: bool,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                default_provider: "ollama".to_string(),
                max_iterations: 50,
                confirm_dangerous_actions: true,
                max_retries: default_max_retries(),
                retry_delay_ms: default_retry_delay_ms(),
                enable_self_correction: default_enable_self_correction(),
            },
            providers: ProvidersConfig {
                ollama: Some(OllamaConfig {
                    host: "http://localhost:11434".to_string(),
                    model: "llava".to_string(),
                }),
                anthropic: None,
                openai: None,
                openrouter: None,
            },
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
            _ => {}
        }
    }
}
