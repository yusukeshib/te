use std::io::{self, IsTerminal, Read};

use anyhow::Result;
use clap::{Parser, Subcommand};

mod app;
mod command;
mod history;
mod shell;
mod tui;

use tui::run_tui;

#[derive(Parser)]
#[command(name = "te")]
#[command(about = "Your helping hand for command-line interfaces", long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(allow_hyphen_values = true)]
    pub wrapped_command: Vec<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize shell integration
    Init {
        /// Shell to generate integration for (zsh, bash, fish)
        shell: String,
        /// Optional key binding for zsh (default: ^T)
        #[arg(short, long)]
        bindkey: Option<String>,
    },
}

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

    let final_command = run_tui(&command_str)?;

    if let Some(cmd) = final_command {
        println!("{}", cmd);
    } else {
        std::process::exit(1);
    }

    Ok(())
}
