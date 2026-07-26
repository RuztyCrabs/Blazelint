//! Rule `invalid-range` — official scan rule `ballerina:12` (Code Smell).

use crate::{
    ast::{Expr, Literal, Stmt},
    config::Config,
    errors::{Diagnostic, DiagnosticKind},
    linter::registry::LintRule,
    linter::visit::walk_exprs,
};

/// Flags a range whose bounds mean it can never iterate, such as `9...0`.
///
/// Ballerina ranges always count upwards, so a start above the end yields an
/// empty sequence — usually a reversed pair of bounds rather than an intent to
/// iterate nothing.
///
/// Only literal bounds are examined. A range over variables cannot be judged
/// without knowing their values, and guessing would produce false positives.
pub struct InvalidRangeRule;

impl LintRule for InvalidRangeRule {
    fn name(&self) -> &'static str {
        "invalid-range"
    }

    fn description(&self) -> &'static str {
        "A range whose start exceeds its end never iterates."
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
            let Expr::Range {
                start,
                end,
                inclusive,
                span,
            } = expr
            else {
                return;
            };
            let (Some(lo), Some(hi)) = (int_literal(start), int_literal(end)) else {
                return;
            };
            // Only a start strictly greater than the end is reported, for both
            // forms. `0..<0` is empty but is a normal way to express "no
            // iterations", so it is not a mistake worth flagging.
            if lo > hi {
                let op = if *inclusive { "..." } else { "..<" };
                diagnostics.push(Diagnostic::new_tracked(
                    DiagnosticKind::Linter,
                    severity,
                    format!(
                        "Range `{lo}{op}{hi}` never iterates because the start exceeds \
                         the end; ranges count upwards."
                    ),
                    span.clone(),
                    line_tracker,
                ));
            }
        });
        diagnostics
    }
}

/// Returns the value of an integer literal, allowing a leading unary minus.
fn int_literal(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Literal {
            value: Literal::Number(n),
            ..
        } if n.fract() == 0.0 => Some(*n as i64),
        Expr::Unary {
            op: crate::ast::UnaryOp::Minus,
            operand,
            ..
        } => int_literal(operand).map(|v| -v),
        Expr::Grouping { expression, .. } => int_literal(expression),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::rules::test_support::run_rule;

    #[test]
    fn flags_descending_range() {
        // The official rule's own noncompliant example (ballerina:12).
        let src = "public function main() {\n   foreach int i in 9...0 {\n   }\n}\n";
        assert_eq!(run_rule(&InvalidRangeRule, src).len(), 1);
    }

    #[test]
    fn accepts_valid_ranges() {
        for src in [
            // The official compliant example.
            "public function main() { foreach int i in 0...9 { } }\n",
            "public function main() { foreach int i in 0..<10 { } }\n",
            // A single-element inclusive range.
            "public function main() { foreach int i in 5...5 { } }\n",
            // An empty half-open range is a normal way to iterate zero times.
            "public function main() { foreach int i in 0..<0 { } }\n",
            // Negative bounds, still ascending.
            "public function main() { foreach int i in -5...5 { } }\n",
        ] {
            assert!(
                run_rule(&InvalidRangeRule, src).is_empty(),
                "should not flag: {src}"
            );
        }
    }

    #[test]
    fn ignores_non_literal_bounds() {
        // Variable bounds cannot be judged without their values.
        let src = "public function main(int a, int b) { foreach int i in a...b { } }\n";
        assert!(run_rule(&InvalidRangeRule, src).is_empty());
    }
}
