//! Rules `isolated-public-function`, `isolated-public-method`, and
//! `isolated-public-class` — official scan rules `ballerina:3`, `:4`, `:5`
//! (Code Smell).
//!
//! Only `isolated` constructs may be called or accessed concurrently. A public
//! declaration that is not `isolated` therefore cannot participate in
//! concurrent code, which is usually not what a public API intends.
//!
//! All three default to `off`. They are advisory: in a codebase that does no
//! concurrent work they would fire on nearly every public declaration, so users
//! opt in.

use crate::{
    ast::Stmt,
    config::Config,
    errors::{Diagnostic, DiagnosticKind, Severity},
    linter::registry::LintRule,
    linter::visit::walk_stmts,
};

/// True when a qualifier list contains `isolated`.
fn is_isolated(qualifiers: &[String]) -> bool {
    qualifiers.iter().any(|q| q == "isolated")
}

fn diagnostic(
    kind: &str,
    name: &str,
    span: &crate::errors::Span,
    severity: Severity,
    line_tracker: &crate::utils::LineTracker,
) -> Diagnostic {
    Diagnostic::new_tracked(
        DiagnosticKind::Linter,
        severity,
        format!(
            "Public {kind} '{name}' is not `isolated`, so it cannot be used concurrently. \
             Mark it `isolated` to allow concurrent access."
        ),
        span.clone(),
        line_tracker,
    )
}

/// `ballerina:3` — a public *module-level* function that is not `isolated`.
pub struct IsolatedPublicFunctionRule;

impl LintRule for IsolatedPublicFunctionRule {
    fn name(&self) -> &'static str {
        "isolated-public-function"
    }

    fn description(&self) -> &'static str {
        "Public functions should be `isolated` so they can be called concurrently."
    }

    fn check(
        &self,
        ast: &[Stmt],
        _file_path: &str,
        _source: &str,
        config: &Config,
        line_tracker: &crate::utils::LineTracker,
    ) -> Vec<Diagnostic> {
        let severity = self.severity(config);
        let mut out = Vec::new();
        // Module level only: a function inside a class is a *method*, covered by
        // the rule below with its own severity.
        for stmt in ast {
            if let Stmt::Function {
                is_public: true,
                qualifiers,
                name,
                name_span,
                ..
            } = stmt
            {
                if !is_isolated(qualifiers) {
                    out.push(diagnostic(
                        "function",
                        name,
                        name_span,
                        severity,
                        line_tracker,
                    ));
                }
            }
        }
        out
    }
}

/// `ballerina:4` — a public method of a class or service that is not `isolated`.
pub struct IsolatedPublicMethodRule;

impl LintRule for IsolatedPublicMethodRule {
    fn name(&self) -> &'static str {
        "isolated-public-method"
    }

    fn description(&self) -> &'static str {
        "Public methods should be `isolated` so they can be called concurrently."
    }

    fn check(
        &self,
        ast: &[Stmt],
        _file_path: &str,
        _source: &str,
        config: &Config,
        line_tracker: &crate::utils::LineTracker,
    ) -> Vec<Diagnostic> {
        let severity = self.severity(config);
        let mut out = Vec::new();
        walk_stmts(ast, &mut |stmt| {
            let (Stmt::ClassDef { members, .. } | Stmt::ServiceDecl { members, .. }) = stmt else {
                return;
            };
            for member in members {
                if let Stmt::Function {
                    is_public: true,
                    qualifiers,
                    name,
                    name_span,
                    ..
                } = member
                {
                    if !is_isolated(qualifiers) {
                        out.push(diagnostic(
                            "method",
                            name,
                            name_span,
                            severity,
                            line_tracker,
                        ));
                    }
                }
            }
        });
        out
    }
}

/// `ballerina:5` — a public class that is not `isolated`.
pub struct IsolatedPublicClassRule;

impl LintRule for IsolatedPublicClassRule {
    fn name(&self) -> &'static str {
        "isolated-public-class"
    }

    fn description(&self) -> &'static str {
        "Public classes should be `isolated` so they can be accessed concurrently."
    }

    fn check(
        &self,
        ast: &[Stmt],
        _file_path: &str,
        _source: &str,
        config: &Config,
        line_tracker: &crate::utils::LineTracker,
    ) -> Vec<Diagnostic> {
        let severity = self.severity(config);
        let mut out = Vec::new();
        walk_stmts(ast, &mut |stmt| {
            if let Stmt::ClassDef {
                is_public: true,
                qualifiers,
                name,
                name_span,
                ..
            } = stmt
            {
                if !is_isolated(qualifiers) {
                    out.push(diagnostic("class", name, name_span, severity, line_tracker));
                }
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::rules::test_support::run_rule;

    #[test]
    fn flags_non_isolated_public_function() {
        // The official rule's own noncompliant example (ballerina:3).
        let src = "public function helperFunction() {\n}\n";
        assert_eq!(run_rule(&IsolatedPublicFunctionRule, src).len(), 1);

        // ...and its compliant counterpart.
        let src = "public isolated function helperFunction() {\n}\n";
        assert!(run_rule(&IsolatedPublicFunctionRule, src).is_empty());

        // A non-public function is not the rule's concern.
        let src = "function helperFunction() {\n}\n";
        assert!(run_rule(&IsolatedPublicFunctionRule, src).is_empty());
    }

    #[test]
    fn flags_non_isolated_public_method() {
        // The official rule's own noncompliant example (ballerina:4).
        let src = "class EvenNumber {\n\
                       int i = 1;\n\
                       public function generate() returns int { return self.i * 2; }\n\
                   }\n";
        assert_eq!(run_rule(&IsolatedPublicMethodRule, src).len(), 1);

        let src = "class EvenNumber {\n\
                       int i = 1;\n\
                       public isolated function generate() returns int { lock { return self.i * 2; } }\n\
                   }\n";
        assert!(run_rule(&IsolatedPublicMethodRule, src).is_empty());

        // A private method is not the rule's concern.
        let src = "class C { function helper() { } }\n";
        assert!(run_rule(&IsolatedPublicMethodRule, src).is_empty());
    }

    #[test]
    fn flags_non_isolated_public_class() {
        // The official rule's own noncompliant example (ballerina:5).
        let src = "public class EvenNumber {\n    int i = 1;\n}\n";
        assert_eq!(run_rule(&IsolatedPublicClassRule, src).len(), 1);

        let src = "public isolated class EvenNumber {\n    int i = 1;\n}\n";
        assert!(run_rule(&IsolatedPublicClassRule, src).is_empty());

        let src = "class EvenNumber {\n    int i = 1;\n}\n";
        assert!(run_rule(&IsolatedPublicClassRule, src).is_empty());
    }

    #[test]
    fn function_rule_does_not_double_report_methods() {
        // A public method must be reported by the method rule only, so the two
        // can be configured independently.
        let src = "public class C { public function m() { } }\n";
        assert!(run_rule(&IsolatedPublicFunctionRule, src).is_empty());
        assert_eq!(run_rule(&IsolatedPublicMethodRule, src).len(), 1);
    }
}
