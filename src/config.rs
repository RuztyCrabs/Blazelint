use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub rules: HashMap<String, RuleSeverity>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub ignore: Ignore,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    Error,
    Warn,
    Info,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    #[serde(default = "default_max_line_length")]
    pub max_line_length: u64,
    #[serde(default = "default_max_function_length")]
    pub max_function_length: u64,
}

#[derive(Debug, Deserialize, Default)]
pub struct Ignore {
    #[serde(default)]
    pub patterns: Vec<String>,
}

fn default_max_line_length() -> u64 {
    120
}

fn default_max_function_length() -> u64 {
    50
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_line_length: default_max_line_length(),
            max_function_length: default_max_function_length(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration file not found")]
    NotFound,
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse configuration file: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

pub fn load_config(start_path: Option<&Path>) -> Result<Config, ConfigError> {
    let path_to_search = match start_path {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir()?,
    };

    find_blazerc(&path_to_search).map_or_else(
        || Ok(Config::default()),
        |path| {
            let content = fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            validate_config(&config)?;
            Ok(config)
        },
    )
}

fn find_blazerc(start_path: &Path) -> Option<PathBuf> {
    let mut current = start_path;

    loop {
        let config_path = current.join(".blazerc");
        if config_path.exists() {
            return Some(config_path);
        }

        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            return None;
        }
    }
}

fn validate_config(_config: &Config) -> Result<(), ConfigError> {
    // TODO: Add validation for unknown rules
    Ok(())
}

impl Default for Config {
    fn default() -> Self {
        let mut rules = HashMap::new();
        rules.insert("camel-case".to_string(), RuleSeverity::Error);
        rules.insert("constant-case".to_string(), RuleSeverity::Warn);
        rules.insert("line-length".to_string(), RuleSeverity::Error);
        rules.insert("snake-case".to_string(), RuleSeverity::Warn);
        rules.insert("unused-variables".to_string(), RuleSeverity::Warn);
        rules.insert("missing-return".to_string(), RuleSeverity::Error);

        Self {
            rules,
            settings: Settings::default(),
            ignore: Ignore::default(),
        }
    }
}
