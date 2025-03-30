use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct LinterConfig {
    #[serde(default)]
    pub rules: HashMap<String, bool>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl Default for LinterConfig {
    fn default() -> Self {
        let mut rules = HashMap::new();
        rules.insert("unknown-token".to_string(), true);
        rules.insert("function-declaration".to_string(), true);
        rules.insert("import-statement".to_string(), true);
        
        LinterConfig {
            rules,
            keywords: vec![
                "import".to_string(),
                "public".to_string(),
                "function".to_string(),
            ],
        }
    }
}

impl LinterConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: LinterConfig = toml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn is_rule_enabled(&self, rule_name: &str) -> bool {
        self.rules.get(rule_name).copied().unwrap_or(true)
    }
}
