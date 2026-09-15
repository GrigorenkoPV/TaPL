use std::fs;

use anyhow::{Context as _, Error};
use culpa::throws;

use crate::parser::parse;

mod ast;
mod cli;
mod parser;

#[throws]
fn main() {
    let cli::Args { input_file } = cli::Args::parse();

    let input = fs::read_to_string(&input_file)
        .with_context(|| format!("Error reading input from {}", input_file.display()))?;
    let program = parse(&input).context("parser error")?;
    println!("{program:?}");
}
