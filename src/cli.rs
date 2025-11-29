use clap::Parser;

#[derive(Parser)]
#[command(name = "te")]
#[command(about = "Your helping hand for command-line interfaces", long_about = None)]
pub struct Cli {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub wrapped_command: Vec<String>,
}
