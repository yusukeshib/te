use anyhow::Result;
use clap::Parser;

mod app;
mod cli;
mod executor;
mod help_parser;
mod tui;

use cli::Cli;
use executor::execute_command;
use tui::run_tui;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.wrapped_command.is_empty() {
        eprintln!("Error: No command specified");
        eprintln!("Usage: te <command> [args...]");
        std::process::exit(1);
    }

    let final_command = run_tui(cli.wrapped_command)?;
    if let Some(cmd) = final_command {
        execute_command(&cmd)?;
    }

    Ok(())
}
