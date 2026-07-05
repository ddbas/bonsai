use clap::{CommandFactory, Parser, Subcommand};
use owo_colors::OwoColorize as _;

use bonsai::worktree;

#[derive(Parser)]
#[command(
    name = "bs",
    about = "🌳 bonsai – instantly provision clean git worktrees so you can context-switch without trashing your working tree.",
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

    /// List all managed worktrees in the pool with their availability status.
    ///
    /// Displays one line per slot: a coloured status badge followed by the
    /// tilde-abbreviated path.  Green = available; red = in use.
    #[command(alias = "ls")]
    List,

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

        Some(Commands::List) => {
            let root = worktree::managed_root()?;
            let slug = worktree::repo_slug()?;
            let pool_dir = root.join(&slug);

            if !pool_dir.exists() {
                println!("No worktrees managed for this repository (pool does not exist yet).");
                println!("Run `bs get` to create the first slot.");
                return Ok(());
            }

            let entries = worktree::list_worktrees_status(&pool_dir)?;

            if entries.is_empty() {
                println!("No worktrees managed for this repository.");
                println!("Run `bs get` to create the first slot.");
                return Ok(());
            }

            for (path, status, process_count) in &entries {
                let tilde = worktree::tilde_path(path);
                match status {
                    worktree::WorktreeStatus::Available => {
                        println!("{}  {}", "available".green(), tilde);
                    }
                    worktree::WorktreeStatus::InUse => match process_count {
                        Some(n) => println!("{}     {}  {:>4} processes", "in use".red(), tilde, n),
                        None => println!("{}     {}", "in use".red(), tilde),
                    },
                }
            }
        }

        Some(Commands::Help) => {
            Cli::command().print_long_help()?;
        }
    }

    Ok(())
}
