use crate::ast::Stmt;
pub use crate::errors::{Diagnostic, Severity}; // Import Severity from errors.rs

/// Common trait for all linting rules.
#[allow(dead_code)]
pub trait LintRule {
    /// Returns a unique name for the rule.
    fn name(&self) -> &'static str;

    /// Describes what the rule does.
    fn description(&self) -> &'static str;

    /// Returns the severity of the rule.
    fn severity(&self) -> Severity {
        Severity::Warning // Default severity
    }

    /// Checks the given abstract syntax tree (AST) for violations of the rule.
    fn check(&self, ast: &[Stmt], file_path: &str, source: &str) -> Vec<Diagnostic>;
}

/// A registry for linting rules.
#[allow(dead_code)]
pub struct LintRuleRegistry {
    rules: Vec<Box<dyn LintRule>>,
    enabled_rules: Vec<String>,
}

#[allow(dead_code)]
impl LintRuleRegistry {
    /// Creates a new, empty lint rule registry.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            enabled_rules: Vec::new(),
        }
    }

    /// Registers a new linting rule.
    pub fn register(&mut self, rule: Box<dyn LintRule>) {
        self.enabled_rules.push(rule.name().to_string()); // Enable all rules by default
        self.rules.push(rule);
    }

    /// Enables a specific rule.
    pub fn enable_rule(&mut self, name: &str) {
        if !self.enabled_rules.contains(&name.to_string()) {
            self.enabled_rules.push(name.to_string());
        }
    }

    /// Disables a specific rule.
    pub fn disable_rule(&mut self, name: &str) {
        self.enabled_rules.retain(|r| r != name);
    }

    /// Runs all enabled linting rules on the given AST.
    pub fn run_all(&self, ast: &[Stmt], file_path: &str, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            if self.enabled_rules.contains(&rule.name().to_string()) {
                diagnostics.extend(rule.check(ast, file_path, source));
            }
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Literal, Stmt};
    use crate::errors::{Diagnostic, DiagnosticKind}; // Removed Position

    // A mock lint rule for testing purposes.
    struct MockRule {
        name: &'static str,
        description: &'static str,
        severity: Severity,
        diagnostics: Vec<Diagnostic>,
    }

    impl LintRule for MockRule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            self.description
        }

        fn severity(&self) -> Severity {
            self.severity
        }

        fn check(&self, _ast: &[Stmt], _file_path: &str, _source: &str) -> Vec<Diagnostic> {
            // When creating diagnostics in the mock rule, ensure they use the rule's severity
            self.diagnostics
                .iter()
                .map(|d| {
                    let mut new_d = d.clone();
                    new_d.severity = self.severity;
                    new_d
                })
                .collect()
        }
    }

    #[test]
    fn test_register_and_run_rule() {
        let mut registry = LintRuleRegistry::new();
        let rule = MockRule {
            name: "mock-rule",
            description: "A mock rule for testing.",
            severity: Severity::Warning,
            diagnostics: vec![Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                Severity::Warning, // Explicitly set severity
                "Mock error".to_string(),
                0..0, // Span is not used in this test, but required
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
        let diagnostics = registry.run_all(&ast, "test.bal", "");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Mock error");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn test_disable_rule() {
        let mut registry = LintRuleRegistry::new();
        let rule = MockRule {
            name: "mock-rule",
            description: "A mock rule for testing.",
            severity: Severity::Warning,
            diagnostics: vec![Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                Severity::Warning, // Explicitly set severity
                "Mock error".to_string(),
                0..0,
            )],
        };

        registry.register(Box::new(rule));
        registry.disable_rule("mock-rule");

        let ast = vec![Stmt::Expression {
            expression: Expr::Literal {
                value: Literal::Number(42.0),
                span: 0..0,
            },
            span: 0..0,
        }];
        let diagnostics = registry.run_all(&ast, "test.bal", "");

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_run_multiple_rules() {
        let mut registry = LintRuleRegistry::new();
        let rule1 = MockRule {
            name: "mock-rule-1",
            description: "A mock rule for testing.",
            severity: Severity::Warning,
            diagnostics: vec![Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                Severity::Warning, // Explicitly set severity
                "Mock error 1".to_string(),
                0..0,
            )],
        };
        let rule2 = MockRule {
            name: "mock-rule-2",
            description: "Another mock rule for testing.",
            severity: Severity::Error,
            diagnostics: vec![Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                Severity::Error, // Explicitly set severity
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
        let diagnostics = registry.run_all(&ast, "test.bal", "");

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().any(|d| d.message == "Mock error 1"));
        assert!(diagnostics.iter().any(|d| d.message == "Mock error 2"));
        assert!(diagnostics.iter().any(|d| d.severity == Severity::Warning));
        assert!(diagnostics.iter().any(|d| d.severity == Severity::Error));
    }
}
