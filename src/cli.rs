use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "te")]
#[command(about = "Your helping hand for command-line interfaces", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub wrapped_command: Vec<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize shell integration
    Init {
        /// Shell to generate integration for (zsh, bash, fish)
        shell: String,
    },
}
