use std::{io, process::ExitCode};

use anyhow::{Context as _, Error};
use culpa::throws;

use crate::parser::parse;

mod ast;
mod parser;
mod typck;

#[throws]
fn main() -> ExitCode {
    let input = io::read_to_string(io::stdin()).context("Error reading input from stdin")?;
    let program = parse(&input).context("parser error")?;
    dbg!(&program);
    match typck::typck(&program) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            let code = e.into();
            debug_assert_ne!(
                code,
                ExitCode::SUCCESS,
                "There was an error, but we have {code:?}?"
            );
            code
        }
    }
}
