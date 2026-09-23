use core::fmt;

use anyhow::{Context as _, anyhow};
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

use crate::{
    ast::Program,
    extensions::{Extension, Extensions},
};

use self::grammar::ProgramParser;

fn flatten(v: Vec<Vec<Extension>>) -> Extensions {
    v.into_iter().flatten().collect()
}

fn parse_extension(extension: &str) -> anyhow::Result<Extension> {
    let extension = extension
        .strip_prefix("#")
        .expect("parser always gives it with `#`");
    extension.parse().map_err(|e| match e {
        strum::ParseError::VariantNotFound => anyhow!("Unknown extension: {extension:?}"),
    })
}

#[expect(clippy::tests_outside_test_module, reason = "sanity check")]
#[expect(clippy::unwrap_used, reason = "this is a test")]
#[test]
fn parse_extensions() {
    let e = Extension::UnitType;
    let s = "unit-type";
    assert_eq!(<&'static str>::from(&e), s);
    assert_eq!(parse_extension(&format!("#{s}")).unwrap(), e);
    assert_eq!(s.parse(), Ok(e));
}

pub(super) struct Position {
    line: usize,
    column: usize,
}
impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { line, column } = self;
        write!(f, "line {line} col {column}")
    }
}
impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Position")
                .field("line", &self.line)
                .field("column", &self.column)
                .finish()
        } else {
            fmt::Display::fmt(&self, f)
        }
    }
}

pub(super) fn parse(
    input: &str,
) -> Result<Program<'_>, ParseError<Position, String, anyhow::Error>> {
    let parser = ProgramParser::new();
    parser.parse(input).map_err(|e| {
        e.map_token(|Token(_, text)| format!("{text:?}"))
            .map_location(|byte_offset| {
                #[expect(clippy::string_slice, reason = "lalrpop gives valid byte offsets")]
                let s = &input[..byte_offset];
                let mut position = match s.rsplit_once('\n') {
                    None => Position {
                        line: 0,
                        column: s.chars().count(),
                    },
                    Some((prev_lines, last_line)) => Position {
                        line: prev_lines.chars().filter(|&c| c == '\n').count() + 1,
                        column: last_line.chars().count(),
                    },
                };
                position.line += 1;
                position.column += 1;
                position
            })
    })
}
