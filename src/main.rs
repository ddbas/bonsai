use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "bs",
    about = "bonsai – project tooling",
    long_about = None,
    // Show help when no subcommand is given.
    arg_required_else_help = true,
    // Disable built-in `help` subcommand so we can define our own.
    disable_help_subcommand = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show usage information
    Help,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Help => {
            Cli::command().print_long_help().unwrap();
        }
    }
}
