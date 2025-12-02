use anyhow::Result;
use clap::Parser;

mod app;
mod cli;
mod command_parser;
mod shell;
mod tui;

use cli::{Cli, Command};
use tui::run_tui;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle init subcommand
    if let Some(Command::Init { shell, bindkey }) = cli.command {
        if let Some(script) = shell::generate_init_script(&shell, bindkey) {
            print!("{}", script);
            return Ok(());
        } else {
            eprintln!("Error: Unsupported shell: {}", shell);
            eprintln!("Supported shells: zsh, bash, fish");
            std::process::exit(1);
        }
    }

    // Handle wrapped command
    if cli.wrapped_command.is_empty() {
        eprintln!("Error: No command specified");
        eprintln!("Usage: te <command> [args...]");
        std::process::exit(1);
    }

    let command_str = cli.wrapped_command.join(" ");
    let final_command = run_tui(command_str)?;

    if let Some(cmd) = final_command {
        println!("{}", cmd);
    } else {
        std::process::exit(1);
    }

    Ok(())
}
