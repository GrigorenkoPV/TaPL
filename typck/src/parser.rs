use anyhow::{Context as _, Error};

use culpa::throws;
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

use self::grammar::ProgramParser;

#[inline]
#[must_use]
fn immortalize_parse_error<L, E>(error: ParseError<L, Token<'_>, E>) -> ParseError<L, String, E> {
    error.map_token(|token| format!("{:?}", token.1))
}

#[throws]
pub(super) fn parse(input: &str) {
    ProgramParser::new()
        .parse(input)
        .map_err(immortalize_parse_error)
        .context("Parse error")?
}
