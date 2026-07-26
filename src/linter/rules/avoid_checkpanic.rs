//! Rule `avoid-checkpanic` — official scan rule `ballerina:1` (Code Smell).

use crate::{
    ast::{Expr, Stmt},
    config::Config,
    errors::{Diagnostic, DiagnosticKind},
    linter::registry::LintRule,
    linter::visit::walk_exprs,
};

/// Flags `checkpanic`, which aborts the program with a panic when the operand
/// evaluates to an error, unless something handles it further up the call stack.
///
/// `check` is almost always the better choice: it returns the error to the
/// caller, or transfers control to an enclosing `on fail` block.
pub struct AvoidCheckpanicRule;

impl LintRule for AvoidCheckpanicRule {
    fn name(&self) -> &'static str {
        "avoid-checkpanic"
    }

    fn description(&self) -> &'static str {
        "Avoid `checkpanic`; propagate the error with `check` or handle it explicitly."
    }

    fn check(
        &self,
        ast: &[Stmt],
        _file_path: &str,
        _source: &str,
        config: &Config,
        line_tracker: &crate::utils::LineTracker,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let severity = self.severity(config);
        walk_exprs(ast, &mut |expr| {
            if let Expr::Check { keyword, span, .. } = expr {
                if keyword == "checkpanic" {
                    diagnostics.push(Diagnostic::new_tracked(
                        DiagnosticKind::Linter,
                        severity,
                        "Avoid `checkpanic`: it panics on error. Use `check` to propagate \
                         the error, or handle it explicitly."
                            .to_string(),
                        span.clone(),
                        line_tracker,
                    ));
                }
            }
        });
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::rules::test_support::run_rule;

    #[test]
    fn flags_checkpanic_but_not_check() {
        // The official rule's own noncompliant example (ballerina:1).
        let noncompliant = "public function checkResult() {\n\
                                json result = checkpanic getResult();\n\
                            }\n";
        assert_eq!(run_rule(&AvoidCheckpanicRule, noncompliant).len(), 1);

        // ...and its compliant counterpart.
        let compliant = "public function checkResult() returns error? {\n\
                             json result = check getResult();\n\
                         }\n";
        assert!(run_rule(&AvoidCheckpanicRule, compliant).is_empty());
    }

    #[test]
    fn reaches_nested_bodies() {
        // Inside an anonymous function — verifies the shared walker descends
        // into statement bodies carried by expressions.
        let src = "function f() {\n\
                       var g = function() { json j = checkpanic h(); };\n\
                   }\n";
        assert_eq!(run_rule(&AvoidCheckpanicRule, src).len(), 1);

        // Inside a class method.
        let src = "class C { function m() { json j = checkpanic h(); } }\n";
        assert_eq!(run_rule(&AvoidCheckpanicRule, src).len(), 1);
    }
}
