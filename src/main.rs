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
    /// ~/.bonsai/<repo>/ (or creates one), resets it to that HEAD, and
    /// prints the absolute path to stdout.
    ///
    /// Use `-b <branch>` to create a new branch at HEAD inside the slot
    /// (fails if the branch already exists, mirroring `git checkout -b`).
    /// Use `-B <branch>` to create or reset a branch (mirroring
    /// `git checkout -B`). Without either flag the slot is left in detached
    /// HEAD state.
    ///
    /// This is also the default command: running `bs` with no subcommand
    /// is equivalent to `bs get` (always detached HEAD; `-b`/`-B` require
    /// the explicit `get` subcommand).
    Get {
        /// Create a new branch at HEAD in the provisioned slot.
        /// Fails if the branch already exists (mirrors `git checkout -b`).
        #[arg(short = 'b', value_name = "BRANCH", conflicts_with = "reset_branch")]
        new_branch: Option<String>,

        /// Create or reset a branch at HEAD in the provisioned slot.
        /// Overwrites an existing branch without error (mirrors `git checkout -B`).
        #[arg(short = 'B', value_name = "BRANCH", conflicts_with = "new_branch")]
        reset_branch: Option<String>,
    },

    /// List all managed worktrees in the pool with their availability status.
    ///
    /// Displays one line per slot: a coloured status badge followed by the
    /// tilde-abbreviated path.  Green = available; red = in use.
    #[command(alias = "ls")]
    List,

    /// Show the managed worktree slot that contains the current directory.
    ///
    /// Prints the tilde-abbreviated path of the bonsai pool slot that the
    /// current working directory lives inside, together with the checked-out
    /// branch name when applicable.
    ///
    /// Exits with status 0 when inside a managed slot; exits with status 1
    /// when the CWD is not part of any managed slot for this repository.
    Current,

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

fn format_current_path(tilde: &str, branch: Option<&str>) -> String {
    match branch {
        Some(b) => format!("{tilde}  ({})", b.bold()),
        None => tilde.to_string(),
    }
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

    // -- column alignment helpers --------------------------------------------

    /// Compute the visible width the same way the rendering loop does.
    fn visible_width(tilde: &str, branch: Option<&str>) -> usize {
        match branch {
            Some(b) => tilde.chars().count() + 3 + b.chars().count(),
            None => tilde.chars().count(),
        }
    }

    #[test]
    fn visible_width_no_branch() {
        // Count manually: "~/.bonsai/bonsai/abc12345" = 25 chars
        assert_eq!(visible_width("~/.bonsai/bonsai/abc12345", None), 25);
    }

    #[test]
    fn visible_width_with_branch() {
        // 25 + " (" (2) + "main" (4) + ")" (1) = 32
        assert_eq!(visible_width("~/.bonsai/bonsai/abc12345", Some("main")), 32);
    }

    #[test]
    fn padding_aligns_stats_column() {
        let short = "~/.bonsai/abc";
        let long = "~/.bonsai/bonsai/abcdef01 (worktree-list-enhancements)";
        let w_short = visible_width(short, None);
        let w_long = visible_width(long, None); // already includes branch in label here
        let max = w_short.max(w_long);
        let pad_short = " ".repeat(max - w_short);
        let pad_long = " ".repeat(max - w_long);
        // Both rows should have path + pad of the same total length.
        assert_eq!(short.len() + pad_short.len(), long.len() + pad_long.len());
    }

    // -- format_current_path -------------------------------------------------

    #[test]
    fn format_current_path_no_branch() {
        let result = format_current_path("~/.bonsai/repo/a3f9c1b2", None);
        assert_eq!(result, "~/.bonsai/repo/a3f9c1b2");
    }

    #[test]
    fn format_current_path_with_branch() {
        // Strip ANSI codes for comparison — bold() wraps with escape sequences.
        let result = format_current_path("~/.bonsai/repo/a3f9c1b2", Some("my-feature"));
        // The result must contain the path and the branch in parentheses.
        assert!(result.contains("~/.bonsai/repo/a3f9c1b2"));
        assert!(result.contains("my-feature"));
        assert!(result.contains('(') && result.contains(')'));
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
        // Default command (no subcommand): always detached HEAD.
        // The -b/-B flags require the explicit `bs get` subcommand.
        None => {
            let path = worktree::get_worktree(None)?;
            println!("🌳 {}", path.display());
        }

        // Explicit `bs get` with optional -b/-B branch flags.
        Some(Commands::Get {
            new_branch,
            reset_branch,
        }) => {
            let branch = match (new_branch, reset_branch) {
                (Some(b), None) => Some(worktree::BranchMode::New(b)),
                (None, Some(b)) => Some(worktree::BranchMode::Reset(b)),
                _ => None,
            };
            // Capture the name before moving `branch` into get_worktree.
            let branch_name: Option<String> = branch.as_ref().map(|m| match m {
                worktree::BranchMode::New(b) | worktree::BranchMode::Reset(b) => b.clone(),
            });
            let path = worktree::get_worktree(branch)?;
            match branch_name.as_deref() {
                Some(b) => println!("🌳 {}  ({})", path.display(), b),
                None => println!("🌳 {}", path.display()),
            }
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

            // Two-pass rendering: collect rows first so we can measure
            // the widest path+branch string and pad all rows to the same
            // column width before printing the stats column.
            struct Row<'a> {
                status: &'a worktree::WorktreeStatus,
                /// Path + optional bold branch — may contain ANSI escape codes.
                path_display: String,
                /// Visible character width of `path_display` (no ANSI codes).
                visible_width: usize,
                stats_str: String,
            }

            let rows: Vec<Row<'_>> = entries
                .iter()
                .map(|(path, status, stats, branch)| {
                    let tilde = worktree::tilde_path(path);
                    // Visible width: tilde chars + " (" + branch + ")" if present.
                    let visible_width = match branch {
                        Some(b) => tilde.chars().count() + 3 + b.chars().count(),
                        None => tilde.chars().count(),
                    };
                    let path_display = match branch {
                        Some(b) => format!("{} ({})", tilde, b.bold()),
                        None => tilde,
                    };
                    Row {
                        status,
                        path_display,
                        visible_width,
                        stats_str: format_stats(stats),
                    }
                })
                .collect();

            let max_width = rows.iter().map(|r| r.visible_width).max().unwrap_or(0);

            for row in &rows {
                let pad = " ".repeat(max_width - row.visible_width);
                match row.status {
                    worktree::WorktreeStatus::Available => {
                        if row.stats_str.is_empty() {
                            println!("{}  {}{}", "available".green(), row.path_display, pad);
                        } else {
                            println!(
                                "{}  {}{}  {}",
                                "available".green(),
                                row.path_display,
                                pad,
                                row.stats_str
                            );
                        }
                    }
                    worktree::WorktreeStatus::InUse => {
                        if row.stats_str.is_empty() {
                            println!("{}     {}{}", "in use".red(), row.path_display, pad);
                        } else {
                            println!(
                                "{}     {}{}  {}",
                                "in use".red(),
                                row.path_display,
                                pad,
                                row.stats_str
                            );
                        }
                    }
                }
            }
        }

        Some(Commands::Current) => match worktree::current_worktree()? {
            Some((path, branch)) => {
                let tilde = worktree::tilde_path(&path);
                println!("🌳 {}", format_current_path(&tilde, branch.as_deref()));
            }
            None => {
                println!("Not inside a managed bonsai worktree.");
                println!("Run `bs get` to provision a slot, then `cd` into it.");
                std::process::exit(1);
            }
        },

        Some(Commands::Help) => {
            Cli::command().print_long_help()?;
        }
    }

    Ok(())
}
