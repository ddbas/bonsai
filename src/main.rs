use clap::{CommandFactory, Parser, Subcommand};

use bonsai::worktree;

#[derive(Parser)]
#[command(
    name = "bs",
    about = "bonsai – project tooling",
    long_about = None,
    // Disable built-in `help` subcommand so we can define our own.
    disable_help_subcommand = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Provision or reuse a managed git worktree from the pool.
    ///
    /// Resolves the current HEAD, finds a clean unlocked slot under
    /// ~/.bonsai/<repo>/ (or creates one), resets it to that HEAD in
    /// detached state, and prints the absolute path to stdout.
    ///
    /// This is also the default command: running `bs` with no subcommand
    /// is equivalent to `bs get`.
    Get,

    /// Show usage information.
    Help,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // Default command (no subcommand) and explicit `bs get` both go here.
        None | Some(Commands::Get) => {
            let path = worktree::get_worktree()?;
            println!("🌳 {}", path.display());
        }

        Some(Commands::Help) => {
            Cli::command().print_long_help()?;
        }
    }

    Ok(())
}
