use std::collections::btree_set::BTreeSet;

use lalrpop_util::{ParseError, lexer::Token};

#[allow(
    // keep-sorted start
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::cast_sign_loss,
    clippy::cloned_instead_of_copied,
    clippy::elidable_lifetime_names,
    clippy::match_same_arms,
    clippy::missing_assert_message,
    clippy::panic,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
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

fn parse_int(int: &str) -> crate::ast::Integer {
    match int.parse() {
        Ok(int) => int,
        Err(e) => todo!("Report: {e:?}"),
    }
}

pub(super) fn parse(input: &str) -> Result<Program<'_>, ParseError<usize, String, anyhow::Error>> {
    let parser = ProgramParser::new();
    parser.parse(input).map_err(immortalize_parse_error)
}
