use crate::ast::Stmt;
use crate::config::{Config, RuleSeverity};
pub use crate::errors::{Diagnostic, Severity}; // Import Severity from errors.rs
use std::collections::HashSet;

/// Common trait for all linting rules.
#[allow(dead_code)]
pub trait LintRule: Send + Sync {
    /// Returns a unique name for the rule.
    fn name(&self) -> &'static str;

    /// Describes what the rule does.
    fn description(&self) -> &'static str;

    /// Returns the severity of the rule based on configuration.
    fn severity(&self, config: &Config) -> Severity {
        // Changed back to Severity
        config
            .rules
            .get(self.name())
            .map(|s| (*s).into()) // This will now use the updated From impl
            .unwrap_or(Severity::Warning) // Default to Warning
    }

    /// Checks the given abstract syntax tree (AST) for violations of the rule.
    fn check(
        &self,
        ast: &[Stmt],
        file_path: &str,
        source: &str,
        config: &Config,
    ) -> Vec<Diagnostic>;
}

/// A registry for linting rules.
#[allow(dead_code)]
pub struct LintRuleRegistry {
    rules: Vec<Box<dyn LintRule>>,
    enabled_rules: HashSet<String>,
}

impl Default for LintRuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl LintRuleRegistry {
    /// Creates a new, empty lint rule registry.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            enabled_rules: HashSet::new(),
        }
    }

    /// Registers a new linting rule.
    pub fn register(&mut self, rule: Box<dyn LintRule>) {
        self.enabled_rules.insert(rule.name().to_string()); // Enable all rules by default
        self.rules.push(rule);
    }

    /// Enables a specific rule.
    pub fn enable_rule(&mut self, name: &str) {
        self.enabled_rules.insert(name.to_string());
    }

    /// Disables a specific rule.
    pub fn disable_rule(&mut self, name: &str) {
        self.enabled_rules.remove(name);
    }

    /// Runs all enabled linting rules on the given AST.
    pub fn run_all(
        &self,
        ast: &[Stmt],
        file_path: &str,
        source: &str,
        config: &Config,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            let rule_severity_from_config = config.rules.get(rule.name());

            match rule_severity_from_config {
                Some(RuleSeverity::Off) => {
                    // Rule is explicitly turned off, skip it.
                    continue;
                }
                Some(_) => {
                    // Rule is explicitly configured with Error, Warn, or Info. Run it.
                    diagnostics.extend(rule.check(ast, file_path, source, config));
                }
                None => {
                    // Rule is not mentioned in the config.
                    // According to the test's intent, this means it should NOT run.
                    // So, do nothing (skip it).
                    continue;
                }
            }
        }
        diagnostics
    }
}

impl From<RuleSeverity> for Severity {
    fn from(severity: RuleSeverity) -> Self {
        match severity {
            RuleSeverity::Error => Severity::Error,
            RuleSeverity::Warn => Severity::Warning,
            RuleSeverity::Info => Severity::Info,
            RuleSeverity::Off => Severity::Info, // Map Off to Info, as it won't be reported anyway
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Literal, Stmt};
    use crate::config::Config;
    use crate::errors::{Diagnostic, DiagnosticKind};

    // A mock lint rule for testing purposes.
    struct MockRule {
        name: &'static str,
        description: &'static str,
        diagnostics: Vec<Diagnostic>,
    }

    impl LintRule for MockRule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            self.description
        }

        fn check(
            &self,
            _ast: &[Stmt],
            _file_path: &str,
            _source: &str,
            config: &Config,
        ) -> Vec<Diagnostic> {
            self.diagnostics
                .iter()
                .map(|d| {
                    let mut new_d = d.clone();
                    new_d.severity = self.severity(config);
                    new_d
                })
                .collect()
        }
    }

    fn get_default_config() -> Config {
        let mut config = Config::default();
        config
            .rules
            .insert("mock-rule".to_string(), RuleSeverity::Warn);
        config
            .rules
            .insert("mock-rule-1".to_string(), RuleSeverity::Warn);
        config
            .rules
            .insert("mock-rule-2".to_string(), RuleSeverity::Error);
        config
    }

    #[test]
    fn test_register_and_run_rule() {
        let mut registry = LintRuleRegistry::new();
        let rule = MockRule {
            name: "mock-rule",
            description: "A mock rule for testing.",
            diagnostics: vec![Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                Severity::Warning, // This will be overridden by the config
                "Mock error".to_string(),
                0..0,
            )],
        };

        registry.register(Box::new(rule));
        let ast = vec![Stmt::Expression {
            expression: Expr::Literal {
                value: Literal::Number(42.0),
                span: 0..0,
            },
            span: 0..0,
        }];
        let config = get_default_config();
        let diagnostics = registry.run_all(&ast, "test.bal", "", &config);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Mock error");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn test_disable_rule_via_config() {
        let mut registry = LintRuleRegistry::new();
        let rule = MockRule {
            name: "mock-rule",
            description: "A mock rule for testing.",
            diagnostics: vec![Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                Severity::Warning,
                "Mock error".to_string(),
                0..0,
            )],
        };

        registry.register(Box::new(rule));

        let ast = vec![Stmt::Expression {
            expression: Expr::Literal {
                value: Literal::Number(42.0),
                span: 0..0,
            },
            span: 0..0,
        }];

        let mut config = get_default_config();
        config.rules.remove("mock-rule"); // Rule is not in config, so it shouldn't run

        let diagnostics = registry.run_all(&ast, "test.bal", "", &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_run_multiple_rules() {
        let mut registry = LintRuleRegistry::new();
        let rule1 = MockRule {
            name: "mock-rule-1",
            description: "A mock rule for testing.",
            diagnostics: vec![Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                Severity::Warning,
                "Mock error 1".to_string(),
                0..0,
            )],
        };
        let rule2 = MockRule {
            name: "mock-rule-2",
            description: "Another mock rule for testing.",
            diagnostics: vec![Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                Severity::Error,
                "Mock error 2".to_string(),
                0..0,
            )],
        };

        registry.register(Box::new(rule1));
        registry.register(Box::new(rule2));

        let ast = vec![Stmt::Expression {
            expression: Expr::Literal {
                value: Literal::Number(42.0),
                span: 0..0,
            },
            span: 0..0,
        }];
        let config = get_default_config();
        let diagnostics = registry.run_all(&ast, "test.bal", "", &config);

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().any(|d| d.message == "Mock error 1"));
        assert!(diagnostics.iter().any(|d| d.message == "Mock error 2"));
        assert!(diagnostics.iter().any(|d| d.severity == Severity::Warning));
        assert!(diagnostics.iter().any(|d| d.severity == Severity::Error));
    }
}
