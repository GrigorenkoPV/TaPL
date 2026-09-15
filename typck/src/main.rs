mod ast;
mod cli;
mod parser;

fn main() {
    let args = cli::Args::parse();
    match args {}
}
