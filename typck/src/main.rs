mod cli;

fn main() {
    let cli::Args {} = cli::Args::parse();
    println!("Hello, world!")
}
