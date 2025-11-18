use blazelint::config::{load_config, RuleSeverity};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_load_default_config() {
    let dir = tempdir().unwrap();
    let config = load_config(Some(dir.path())).unwrap();
    assert_eq!(config.settings.max_line_length, 120);
    assert_eq!(config.settings.max_function_length, 50);
    assert_eq!(config.rules.get("camel-case"), Some(&RuleSeverity::Error));
}

#[test]
fn test_load_custom_config() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".blazerc");
    let config_content = r#"
[rules]
camel-case = "warn"
line-length = "info"

[settings]
max-line-length = 100
max-function-length = 40

[ignore]
patterns = ["vendor/**"]
"#;
    fs::write(config_path, config_content).unwrap();

    let config = load_config(Some(dir.path())).unwrap();

    assert_eq!(config.settings.max_line_length, 100);
    assert_eq!(config.settings.max_function_length, 40);
    assert_eq!(config.rules.get("camel-case"), Some(&RuleSeverity::Warn));
    assert_eq!(config.rules.get("line-length"), Some(&RuleSeverity::Info));
    assert_eq!(config.ignore.patterns, vec!["vendor/**"]);
}
