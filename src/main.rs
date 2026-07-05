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

fn format_stats(stats: &worktree::WorktreeStats) -> String {
    let mut parts: Vec<String> = Vec::new();
    if stats.process_count > 0 {
        parts.push(format!("\u{2699}{}", stats.process_count)); // ⚙
    }
    if stats.uncommitted_count > 0 {
        parts.push(format!("\u{00b1}{}", stats.uncommitted_count)); // ±
    }
    if stats.untracked_count > 0 {
        parts.push(format!("?{}", stats.untracked_count));
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bonsai::worktree::WorktreeStats;

    fn stats(
        process_count: usize,
        uncommitted_count: usize,
        untracked_count: usize,
    ) -> WorktreeStats {
        WorktreeStats {
            process_count,
            uncommitted_count,
            untracked_count,
        }
    }

    #[test]
    fn format_stats_all_zero_is_empty() {
        assert_eq!(format_stats(&stats(0, 0, 0)), "");
    }

    #[test]
    fn format_stats_all_three_non_zero() {
        assert_eq!(format_stats(&stats(1, 2, 3)), "\u{2699}1 \u{00b1}2 ?3");
    }

    #[test]
    fn format_stats_only_processes() {
        assert_eq!(format_stats(&stats(5, 0, 0)), "\u{2699}5");
    }

    #[test]
    fn format_stats_only_uncommitted() {
        assert_eq!(format_stats(&stats(0, 3, 0)), "\u{00b1}3");
    }

    #[test]
    fn format_stats_only_untracked() {
        assert_eq!(format_stats(&stats(0, 0, 4)), "?4");
    }

    #[test]
    fn format_stats_processes_and_untracked_skip_uncommitted() {
        assert_eq!(format_stats(&stats(2, 0, 4)), "\u{2699}2 ?4");
    }
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

            for (path, status, stats, branch) in &entries {
                let tilde = worktree::tilde_path(path);
                let path_display = match branch {
                    Some(b) => format!("{} ({})", tilde, b.bold()),
                    None => tilde,
                };
                let stats_str = format_stats(stats);
                match status {
                    worktree::WorktreeStatus::Available => {
                        println!("{}  {}", "available".green(), path_display);
                    }
                    worktree::WorktreeStatus::InUse => {
                        if stats_str.is_empty() {
                            println!("{}     {}", "in use".red(), path_display);
                        } else {
                            println!("{}     {}  {}", "in use".red(), path_display, stats_str);
                        }
                    }
                }
            }
        }

        Some(Commands::Help) => {
            Cli::command().print_long_help()?;
        }
    }

    Ok(())
}
