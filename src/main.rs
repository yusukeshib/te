use anyhow::Result;
use clap::Parser;
use std::io::{self, IsTerminal, Read};

mod app;
mod cli;
mod command_parser;
mod history;
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
    let command_str = if cli.wrapped_command.is_empty() {
        // Check if stdin is piped (not a terminal)
        let stdin = io::stdin();
        if !stdin.is_terminal() {
            // Read from stdin
            let mut buffer = String::new();
            stdin.lock().read_to_string(&mut buffer)?;
            buffer.trim().to_string()
        } else {
            eprintln!("Error: No command specified");
            eprintln!("Usage: te <command> [args...]");
            eprintln!("       echo '<command>' | te");
            std::process::exit(1);
        }
    } else {
        cli.wrapped_command.join(" ")
    };

    let final_command = run_tui(command_str)?;

    if let Some(cmd) = final_command {
        println!("{}", cmd);
    } else {
        std::process::exit(1);
    }

    Ok(())
}
