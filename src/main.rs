use anyhow::Result;
use clap::Parser;

mod app;
mod cli;
mod command_parser;
mod tui;

use cli::Cli;
use tui::run_tui;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.wrapped_command.is_empty() {
        eprintln!("Error: No command specified");
        eprintln!("Usage: te <command> [args...]");
        std::process::exit(1);
    }

    let command_str = cli.wrapped_command.join(" ");
    let final_command = run_tui(command_str)?;

    if let Some(cmd) = final_command {
        println!("\n{}", cmd);
    }

    Ok(())
}
