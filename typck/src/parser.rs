use std::collections::btree_set::BTreeSet;

use lalrpop_util::{ParseError, lexer::Token};

#[allow(
    // keep-sorted start
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::as_conversions,
    clippy::deprecated_cfg_attr,
    clippy::just_underscores_and_digits,
    clippy::let_unit_value,
    clippy::missing_assert_message,
    clippy::needless_lifetimes,
    clippy::panic,
    clippy::pedantic,
    clippy::unwrap_used,
    dead_code,
    single_use_lifetimes,
    unit_bindings,
    unreachable_pub,
    unused_imports,
    unused_qualifications,
    // keep-sorted end
    reason = "generated code"
)]
mod grammar;

use crate::ast::Program;

use self::grammar::ProgramParser;

fn flatten<T: Ord>(v: Vec<Vec<T>>) -> BTreeSet<T> {
    v.into_iter().flatten().collect()
}

#[inline]
#[must_use]
fn immortalize_parse_error<L, E>(error: ParseError<L, Token<'_>, E>) -> ParseError<L, String, E> {
    error.map_token(|token| format!("{:?}", token.1))
}

pub(super) fn parse(input: &str) -> Result<Program<'_>, ParseError<usize, String, anyhow::Error>> {
    let parser = ProgramParser::new();
    parser.parse(input).map_err(immortalize_parse_error)
}
