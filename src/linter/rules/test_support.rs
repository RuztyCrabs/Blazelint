//! Shared helpers for lint-rule unit tests.
//!
//! Lets a rule test state a source snippet and the diagnostics expected from
//! it, without each test repeating the lex/parse/config setup.

#![cfg(test)]

use crate::config::{Config, RuleSeverity};
use crate::errors::Diagnostic;
use crate::lexer::Lexer;
use crate::linter::registry::LintRule;
use crate::parser::Parser;
use crate::utils::LineTracker;

/// Runs a single rule over `source` and returns its diagnostics.
///
/// The rule is enabled at `Warn` regardless of any on-disk configuration, so a
/// test exercises the rule itself rather than the ambient `.blazerc`.
pub fn run_rule(rule: &dyn LintRule, source: &str) -> Vec<Diagnostic> {
    let tokens = Lexer::new(source)
        .collect::<Result<Vec<_>, _>>()
        .expect("test source should lex");
    let (ast, parse_errors) = Parser::new(tokens).parse();
    assert!(
        parse_errors.is_empty(),
        "test source should parse, got: {parse_errors:?}"
    );

    let mut config = Config::default();
    config
        .rules
        .insert(rule.name().to_string(), RuleSeverity::Warn);
    let tracker = LineTracker::new(source);
    rule.check(&ast, "test.bal", source, &config, &tracker)
}
