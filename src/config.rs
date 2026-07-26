use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Deserialize, Clone)]
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
    Off, // Added 'Off' to allow disabling rules
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    #[serde(default = "default_max_line_length")]
    pub max_line_length: u64,
    #[serde(default = "default_max_function_length")]
    pub max_function_length: u64,
}

#[derive(Debug, Deserialize, Default, Clone)]
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

#[derive(Debug, Error)] // Removed Clone
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

// Static cache for the default config (when start_path is None)
static CACHED_DEFAULT_CONFIG: Lazy<Result<Config, ConfigError>> = Lazy::new(|| {
    let path_to_search = std::env::current_dir().map_err(ConfigError::ReadError)?;
    _load_config_from_path(&path_to_search)
});

// Helper function to perform the actual loading logic
fn _load_config_from_path(start_path: &Path) -> Result<Config, ConfigError> {
    find_blazerc(start_path).map_or_else(
        || Ok(Config::default()),
        |path| {
            let content = fs::read_to_string(path).map_err(ConfigError::ReadError)?;
            let config: Config = toml::from_str(&content).map_err(ConfigError::ParseError)?;
            validate_config(&config)?;
            Ok(config)
        },
    )
}

pub fn load_config(start_path: Option<&Path>) -> Result<Config, ConfigError> {
    match start_path {
        None => {
            // Access the cached result. If it's an error, convert it to a new ConfigError.
            Ok(CACHED_DEFAULT_CONFIG
                .as_ref()
                .map_err(|e| ConfigError::ValidationError(e.to_string()))?
                .clone())
        }
        Some(p) => {
            // Load config dynamically for a specific path (not cached in CACHED_DEFAULT_CONFIG)
            _load_config_from_path(p)
        }
    }
}

// Static cache for find_blazerc results
static BLAZERC_PATH_CACHE: Lazy<Mutex<HashMap<PathBuf, Option<PathBuf>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn find_blazerc(start_path: &Path) -> Option<PathBuf> {
    let mut cache = BLAZERC_PATH_CACHE.lock().unwrap();

    if let Some(cached_result) = cache.get(start_path) {
        return cached_result.clone();
    }

    let mut current = start_path;
    let mut found_path = None;

    loop {
        let config_path = current.join(".blazerc");
        if config_path.exists() {
            found_path = Some(config_path);
            break;
        }

        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    cache.insert(start_path.to_path_buf(), found_path.clone());
    found_path
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
        rules.insert("unused-variables".to_string(), RuleSeverity::Warn);
        // Off by default: a parameter is part of a signature, so callbacks and
        // overridden methods cannot drop one even when the body ignores it.
        rules.insert("unused-parameters".to_string(), RuleSeverity::Off);
        rules.insert("missing-return".to_string(), RuleSeverity::Error);
        rules.insert("max-function-length".to_string(), RuleSeverity::Warn);

        Self {
            rules,
            settings: Settings::default(),
            ignore: Ignore::default(),
        }
    }
}
