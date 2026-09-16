use std::io;

use anyhow::{Context as _, Error};
use culpa::throws;

use crate::parser::parse;

mod ast;
mod parser;

#[throws]
fn main() {
    let input = io::read_to_string(io::stdin()).context("Error reading input from stdin")?;
    let program = parse(&input).context("parser error")?;
    dbg!(&program);
}
