use core::num::NonZero;
use std::{
    io,
    panic::{self, PanicHookInfo},
    process::ExitCode,
};

use anyhow::{Context as _, Error};
use culpa::throws;

use crate::parser::parse;

mod ast;
mod extensions;
mod namespace;
mod parser;
mod typck;

#[cfg(test)]
mod tests;

#[throws]
fn run_typck() -> Result<(), NonZero<u8>> {
    let input = io::read_to_string(io::stdin()).context("Error reading input from stdin")?;
    let program = parse(&input).context("parser error")?;
    typck::typck(&program).map_err(|e| {
        println!("{e}");
        e.code()
    })
}

fn add_panic_hook(hook: impl Fn(&PanicHookInfo<'_>) + Sync + Send + 'static) {
    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        hook(panic_info);
        prev(panic_info);
    }));
}

fn main() -> ExitCode {
    add_panic_hook(|_| eprintln!("Internal error:"));
    match run_typck() {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(exit_code)) => ExitCode::from(exit_code.get()),
        Err(internal_err) => {
            eprintln!("Internal error:\n{internal_err:?}");
            ExitCode::from(u8::MAX)
        }
    }
}
