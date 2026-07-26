//! Rule `self-assignment` — official scan rule `ballerina:10` (Code Smell).

use crate::{
    ast::{Expr, Stmt},
    config::Config,
    errors::{Diagnostic, DiagnosticKind},
    linter::registry::LintRule,
    linter::visit::walk_exprs,
};

/// Flags assigning a variable to itself (`x = x`, `self.count = self.count`).
///
/// Such an assignment does not change any state, so it is either redundant or —
/// more often — a symptom of incomplete logic, such as a mistyped operand.
pub struct SelfAssignmentRule;

impl LintRule for SelfAssignmentRule {
    fn name(&self) -> &'static str {
        "self-assignment"
    }

    fn description(&self) -> &'static str {
        "A variable assigned to itself has no effect and usually indicates a mistake."
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
            let redundant = match expr {
                // `x = x`
                Expr::Assign { name, value, .. } => {
                    matches!(value.as_ref(), Expr::Variable { name: v, .. } if v == name)
                }
                // `self.f = self.f`, `a[0] = a[0]`
                Expr::MemberAssign { target, value, .. } => same_lvalue(target, value),
                _ => false,
            };
            if redundant {
                diagnostics.push(Diagnostic::new_tracked(
                    DiagnosticKind::Linter,
                    severity,
                    "This variable is assigned to itself; the assignment has no effect."
                        .to_string(),
                    expr.span().clone(),
                    line_tracker,
                ));
            }
        });
        diagnostics
    }
}

/// Compares two lvalue expressions structurally, ignoring spans.
///
/// Only the shapes that can appear on both sides of an assignment are compared;
/// anything else (a call, an arithmetic expression) is treated as different,
/// since it may have side effects or a differing value.
fn same_lvalue(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Variable { name: x, .. }, Expr::Variable { name: y, .. }) => x == y,
        (
            Expr::FieldAccess {
                object: ao,
                field: af,
                ..
            },
            Expr::FieldAccess {
                object: bo,
                field: bf,
                ..
            },
        ) => af == bf && same_lvalue(ao, bo),
        (
            Expr::MemberAccess {
                object: ao,
                member: am,
                ..
            },
            Expr::MemberAccess {
                object: bo,
                member: bm,
                ..
            },
        ) => same_lvalue(ao, bo) && same_lvalue(am, bm),
        // Literals and plain variable reads are side-effect free, so evaluating
        // them twice yields the same value and the assignment is redundant.
        // Anything else (a call, arithmetic) is treated as different.
        (Expr::Literal { value: x, .. }, Expr::Literal { value: y, .. }) => {
            format!("{x:?}") == format!("{y:?}")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::rules::test_support::run_rule;

    #[test]
    fn flags_simple_self_assignment() {
        // The official rule's own noncompliant example (ballerina:10).
        let src = "public function main() {\n   int x = 5;\n   x = x;\n}\n";
        assert_eq!(run_rule(&SelfAssignmentRule, src).len(), 1);
    }

    #[test]
    fn flags_field_and_index_self_assignment() {
        let src = "class C { int count = 0; function m() { self.count = self.count; } }\n";
        assert_eq!(run_rule(&SelfAssignmentRule, src).len(), 1);

        let src = "function f(int[] a) { a[0] = a[0]; }\n";
        assert_eq!(run_rule(&SelfAssignmentRule, src).len(), 1);

        // A variable index is also comparable: reading `i` has no side effects,
        // so `a[i] = a[i]` is equally redundant.
        let src = "function f(int[] a, int i) { a[i] = a[i]; }\n";
        assert_eq!(run_rule(&SelfAssignmentRule, src).len(), 1);
    }

    #[test]
    fn ignores_genuine_assignments() {
        for src in [
            "function f() { int x = 5; int y = 1; x = y; }\n",
            "function f() { int x = 5; x = x + 1; }\n",
            "class C { int a = 0; int b = 0; function m() { self.a = self.b; } }\n",
            "function f(int[] a) { a[0] = a[1]; }\n",
            "function f(int[] a, int i, int j) { a[i] = a[j]; }\n",
        ] {
            assert!(
                run_rule(&SelfAssignmentRule, src).is_empty(),
                "should not flag: {src}"
            );
        }
    }
}
