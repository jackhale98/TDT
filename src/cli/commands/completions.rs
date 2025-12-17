//! Shell completion generation
//!
//! Generates shell completion scripts for bash, zsh, fish, and PowerShell.
//!
//! # Usage
//!
//! ```bash
//! # Bash - add to ~/.bashrc
//! source <(tdt completions bash)
//!
//! # Zsh - add to ~/.zshrc
//! source <(tdt completions zsh)
//!
//! # Fish - add to ~/.config/fish/completions/tdt.fish
//! tdt completions fish > ~/.config/fish/completions/tdt.fish
//!
//! # PowerShell - add to $PROFILE
//! tdt completions powershell >> $PROFILE
//! ```

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use miette::Result;
use std::io;

use crate::cli::Cli;

#[derive(clap::Args, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

pub fn run(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    generate(args.shell, &mut cmd, "tdt", &mut io::stdout());
    Ok(())
}
