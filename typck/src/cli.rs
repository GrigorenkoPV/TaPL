use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, about, help_expected = true)]
pub(crate) struct Args {
    /// Path to the file with the source code
    pub input_file: PathBuf,
}

impl Args {
    #[must_use]
    #[expect(clippy::same_name_method, reason = "hiding clap under the rug")]
    pub(crate) fn parse() -> Self {
        <Self as Parser>::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Args::command().debug_assert()
    }
}
