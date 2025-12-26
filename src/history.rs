use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use crate::app::CommandComponent;
use crate::command_parser::parse_command;

pub enum Shell {
    Zsh,
    Bash,
    Fish,
}

// Detect shell
pub fn detect_shell() -> Shell {
    let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    if shell_path.contains("zsh") {
        Shell::Zsh
    } else if shell_path.contains("fish") {
        Shell::Fish
    } else {
        Shell::Bash
    }
}

// Get history file path
fn get_history_file(shell: &Shell) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let home_path = PathBuf::from(home);

    match shell {
        Shell::Bash => Some(home_path.join(".bash_history")),
        Shell::Zsh => {
            if let Ok(histfile) = std::env::var("HISTFILE") {
                Some(PathBuf::from(histfile))
            } else {
                Some(home_path.join(".zsh_history"))
            }
        }
        Shell::Fish => Some(home_path.join(".local/share/fish/fish_history")),
    }
}

// Parse history and return Vec
fn parse_history_lines(shell: Shell, reader: BufReader<File>) -> Vec<String> {
    match shell {
        Shell::Bash => {
            // Each line is one command
            reader
                .lines()
                .filter_map(|line| line.ok())
                .filter(|line| !line.trim().is_empty())
                .collect()
        }
        Shell::Zsh => {
            // Support both simple and extended formats
            reader
                .lines()
                .filter_map(|line| line.ok())
                .filter_map(|line| {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        return None;
                    }
                    // Extended format: ": timestamp:duration;command"
                    if let Some(stripped) = trimmed.strip_prefix(':') {
                        if let Some(semicolon_pos) = stripped.find(';') {
                            return Some(stripped[semicolon_pos + 1..].to_string());
                        }
                    }
                    // Simple format
                    Some(trimmed.to_string())
                })
                .collect()
        }
        Shell::Fish => {
            // YAML-like format: "- cmd: " or "  cmd: "
            reader
                .lines()
                .filter_map(|line| line.ok())
                .filter_map(|line| {
                    let trimmed = line.trim();
                    if let Some(cmd) = trimmed.strip_prefix("- cmd: ") {
                        Some(cmd.to_string())
                    } else if let Some(cmd) = trimmed.strip_prefix("cmd: ") {
                        Some(cmd.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        }
    }
}

// Filter by base command
fn matches_base_command(command: &str, base_command: &[String]) -> bool {
    let tokens = match shlex::split(command) {
        Some(t) => t,
        None => return false,
    };

    if tokens.len() < base_command.len() {
        return false;
    }

    tokens
        .iter()
        .take(base_command.len())
        .zip(base_command.iter())
        .all(|(a, b)| a == b)
}

// Main function: Load history and return value candidates for each flag
pub fn load_history_for_command(base_command: &[String]) -> Result<HashMap<String, Vec<String>>> {
    let shell = detect_shell();

    let history_file = match get_history_file(&shell) {
        Some(f) => f,
        None => return Ok(HashMap::new()),
    };

    if !history_file.exists() {
        return Ok(HashMap::new());
    }

    let file = File::open(&history_file)?;
    let reader = BufReader::new(file);

    let commands = parse_history_lines(shell, reader);

    // Set of values for each flag
    let mut values: HashMap<String, HashSet<String>> = HashMap::new();

    let max_commands = 100000;
    let mut count = 0;

    for command in commands.iter().rev() {
        // Process from newest commands
        if !matches_base_command(command, base_command) {
            continue;
        }

        if let Ok(components) = parse_command(command) {
            for component in components {
                if let CommandComponent::StringArgument(flag, value) = component {
                    values
                        .entry(flag)
                        .or_insert_with(HashSet::new)
                        .insert(value);
                }
            }
        }

        count += 1;
        if count >= max_commands {
            break;
        }
    }

    // Convert HashSet -> Vec and sort
    let mut result = HashMap::new();
    for (flag, value_set) in values {
        let mut value_vec: Vec<_> = value_set.into_iter().collect();
        value_vec.sort();
        result.insert(flag, value_vec);
    }

    Ok(result)
}
