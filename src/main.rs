use clap::{CommandFactory, Parser, Subcommand};
use owo_colors::OwoColorize as _;

use bonsai::logging;
use bonsai::tmux;
use bonsai::worktree;

#[derive(Parser)]
#[command(
    name = "bs",
    about = "🌳 bonsai – provision clean git worktrees for fast context-switching.",
    long_about = None,
    // Disable built-in `help` subcommand so we can define our own.
    disable_help_subcommand = true,
)]
struct Cli {
    /// Log level for the log file: trace, debug, info, warn, or error
    /// (default: info). Does not affect stdout/stderr output.
    #[arg(long, global = true, default_value = "info", value_name = "LEVEL")]
    log_level: logging::LogLevel,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Provision or reuse a managed git worktree from the pool, printing its
    /// path.
    ///
    /// Resets the slot to the current HEAD. Use `-b <branch>` to create a new
    /// branch, `-B <branch>` to create-or-reset a branch, or pass a
    /// positional `<branch>` to check out an existing branch; these three
    /// are mutually exclusive, and with none the slot is left in detached
    /// HEAD state.
    ///
    /// This is the implicit default command: running `bs` alone is
    /// equivalent to `bs get` in detached HEAD state.
    Get {
        /// Check out an existing branch inside the provisioned slot. Fails
        /// if the branch does not exist. Mutually exclusive with `-b`/`-B`.
        #[arg(
            value_name = "BRANCH",
            conflicts_with = "new_branch",
            conflicts_with = "reset_branch"
        )]
        branch: Option<String>,

        /// Create a new branch at HEAD in the provisioned slot. Fails if the
        /// branch already exists. Mutually exclusive with `-B`.
        #[arg(short = 'b', value_name = "BRANCH", conflicts_with = "reset_branch")]
        new_branch: Option<String>,

        /// Create or reset a branch at HEAD in the provisioned slot,
        /// overwriting an existing branch without error. Mutually exclusive
        /// with `-b`.
        #[arg(short = 'B', value_name = "BRANCH", conflicts_with = "new_branch")]
        reset_branch: Option<String>,

        /// Create (or reuse) a tmux session rooted at the provisioned slot.
        /// With no value, a session name is generated automatically; pass
        /// `--tmux-session=NAME` to use a specific name. Requires `tmux` on
        /// `PATH`.
        #[arg(
            long = "tmux-session",
            value_name = "NAME",
            num_args = 0..=1,
            default_missing_value = ""
        )]
        tmux_session: Option<String>,

        /// Create the tmux session in the background without attaching the
        /// invoking terminal to it. Requires `--tmux-session`.
        #[arg(long = "no-attach", requires = "tmux_session")]
        no_attach: bool,
    },

    /// List all managed worktrees in the pool.
    ///
    /// Displays one line per slot: a coloured status badge
    /// (`available`/`in use`/`locked`), followed by the tilde-abbreviated
    /// path, optionally followed by the checked-out branch name in
    /// parentheses. The current slot is marked with a `▶` prefix (no other
    /// worktrees are so marked). Per-slot
    /// checks short-circuit as soon as the badge's classification is known:
    /// a locked slot never triggers `git status`/`lsof`; a dirty unlocked
    /// slot never triggers `lsof`. No per-file or per-process detail (counts
    /// or itemized lists) is computed or displayed here — use
    /// `bs status [PATH]` for that.
    #[command(alias = "ls")]
    List,

    /// Show detailed status for a single managed worktree slot.
    ///
    /// Reports the slot's overall classification (`available` / `in use` /
    /// `locked`, using the same priority rules `bs list`'s status badge
    /// applies), plus itemized detail: the lock reason (if locked), the
    /// individual `git status --porcelain` lines (uncommitted and untracked),
    /// and the PID + command name of each process with an open file handle
    /// directly at the slot root.
    ///
    /// `PATH` is optional; when omitted, `bs status` resolves the slot
    /// containing the current working directory (same resolution as
    /// `bs current`) and errors out if the CWD is not inside a managed slot.
    /// When given, `PATH` must resolve to a bonsai-managed pool slot for the
    /// current repository (validated the same way as `bs lock`/`bs unlock`).
    Status {
        /// Absolute path to the pool slot to inspect.
        /// Defaults to the current slot when omitted.
        path: Option<std::path::PathBuf>,
    },

    /// Show the managed worktree slot that contains the current directory.
    ///
    /// Exits with status 0 when inside a managed slot, 1 otherwise.
    Current,

    /// Show usage information.
    Help,

    /// Lock a bonsai pool slot, preventing `bs get` from reusing it.
    ///
    /// Defaults to the current slot when no path argument is supplied.
    Lock {
        /// Reason stored with the lock, forwarded to git verbatim.
        #[arg(long, value_name = "MESSAGE")]
        reason: Option<String>,

        /// Path to the pool slot to lock. Defaults to the current slot.
        path: Option<std::path::PathBuf>,
    },

    /// Unlock a bonsai pool slot, making it available for reuse by `bs get`.
    ///
    /// Defaults to the current slot when no path argument is supplied.
    Unlock {
        /// Path to the pool slot to unlock. Defaults to the current slot.
        path: Option<std::path::PathBuf>,
    },

    /// Print bonsai's runtime paths and metadata.
    ///
    /// Prints resolved paths (log directory, current log file, managed root)
    /// and metadata (version, effective log level) as `key: value` lines.
    /// Performs no filesystem writes.
    Info,

    /// Reclaim disk space by deleting unused (`available`) bonsai pool slots
    /// for the current repository, then running `git worktree prune`.
    ///
    /// Classifies every managed pool slot using the same `available` /
    /// `in use` / `locked` rules as `bs list`/`bs status`. Only slots
    /// classified `available` have their on-disk directory deleted; `locked`
    /// and `in use` slots are never touched. After deleting, `git worktree
    /// prune` is run exactly once so git deregisters the removed slots — `bs
    /// prune` never edits git's worktree bookkeeping directly. Prints the
    /// tilde-abbreviated path (and branch, if any) of each pruned slot.
    /// Exits 0 unless at least one slot's directory failed to delete.
    Prune,
}

fn format_current_path(tilde: &str, branch: Option<&str>) -> String {
    match branch {
        Some(b) => format!("{tilde}  ({})", b.bold()),
        None => tilde.to_string(),
    }
}

/// Render a [`worktree::SlotStatusReport`] to stdout in the plain-text,
/// itemized format described by `bs status`'s spec: a header line with the
/// path/branch, a classification line (with lock reason when locked), and
/// itemized sections for open processes, uncommitted files, and untracked
/// files (omitted when empty).
fn print_status_report(report: &worktree::SlotStatusReport) {
    let tilde = worktree::tilde_path(&report.path);
    println!(
        "🌳 {}",
        format_current_path(&tilde, report.branch.as_deref())
    );

    match report.status {
        worktree::WorktreeStatus::Locked => {
            println!("status: {}", "locked".yellow());
            match &report.lock_reason {
                Some(reason) => println!("lock reason: {reason}"),
                None => println!("lock reason: none"),
            }
        }
        worktree::WorktreeStatus::InUse => println!("status: {}", "in use".red()),
        worktree::WorktreeStatus::Available => println!("status: {}", "available".green()),
    }

    if !report.processes.is_empty() {
        println!();
        println!("open processes:");
        for process in &report.processes {
            println!("  {}  {}", process.pid, process.command);
        }
    }

    if !report.git_status.uncommitted.is_empty() {
        println!();
        println!(
            "uncommitted changes ({}):",
            report.git_status.uncommitted.len()
        );
        for line in &report.git_status.uncommitted {
            println!("  {}  {}", line.code, line.path);
        }
    }

    if !report.git_status.untracked.is_empty() {
        println!();
        println!("untracked files ({}):", report.git_status.untracked.len());
        for line in &report.git_status.untracked {
            println!("  {}  {}", line.code, line.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- column alignment helpers --------------------------------------------

    /// Compute the visible width the same way the rendering loop does.
    fn visible_width(tilde: &str, branch: Option<&str>) -> usize {
        match branch {
            Some(b) => tilde.chars().count() + 3 + b.chars().count(),
            None => tilde.chars().count(),
        }
    }

    /// Build the path_display string the same way the rendering loop does.
    fn make_path_display(tilde: &str, branch: Option<&str>) -> String {
        match branch {
            Some(b) => format!("{} ({})", tilde, b),
            None => tilde.to_string(),
        }
    }

    #[test]
    fn visible_width_no_branch() {
        assert_eq!(visible_width("~/.bonsai/bonsai/abc12345", None), 25);
    }

    #[test]
    fn visible_width_with_branch() {
        assert_eq!(visible_width("~/.bonsai/bonsai/abc12345", Some("main")), 32);
    }

    #[test]
    fn padding_aligns_stats_column() {
        let short = "~/.bonsai/abc";
        let long = "~/.bonsai/bonsai/abcdef01 (worktree-list-enhancements)";
        let w_short = visible_width(short, None);
        let w_long = visible_width(long, None);
        let max = w_short.max(w_long);
        let pad_short = " ".repeat(max - w_short);
        let pad_long = " ".repeat(max - w_long);
        assert_eq!(short.len() + pad_short.len(), long.len() + pad_long.len());
    }

    // -- current-slot indicator in list output --------------------------------

    #[test]
    fn current_slot_path_display_no_branch() {
        // Arrow-only mode: no (current) label, path display is same as non-current.
        let display = make_path_display("~/.bonsai/repo/a3f9c1b2", None);
        assert!(
            !display.contains("(current)"),
            "no (current) label expected, got: {display}"
        );
        assert!(display.contains("~/.bonsai/repo/a3f9c1b2"));
    }

    #[test]
    fn current_slot_path_display_with_branch() {
        let display = make_path_display("~/.bonsai/repo/a3f9c1b2", Some("my-feature"));
        assert!(
            !display.contains("(current)"),
            "no (current) label expected, got: {display}"
        );
        assert!(display.contains("my-feature"));
        assert!(display.contains("~/.bonsai/repo/a3f9c1b2"));
    }

    #[test]
    fn non_current_slot_path_display_has_no_current_label() {
        let display = make_path_display("~/.bonsai/repo/b4e8d2f1", Some("main"));
        assert!(
            !display.contains("(current)"),
            "non-current row must not have (current), got: {display}"
        );
    }

    #[test]
    fn current_and_non_current_same_visible_width() {
        // With arrow-only, is_current doesn't affect width.
        let w = visible_width("~/.bonsai/repo/abc", None);
        assert_eq!(w, "~/.bonsai/repo/abc".chars().count());
    }

    #[test]
    fn current_slot_column_alignment_preserved() {
        let current_tilde = "~/.bonsai/repo/abc";
        let other_tilde = "~/.bonsai/repo/much-longer-slot";
        let w_current = visible_width(current_tilde, None);
        let w_other = visible_width(other_tilde, None);
        let max = w_current.max(w_other);
        assert!(max >= w_current);
        assert!(max >= w_other);
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

    // Initialize logging (best-effort; failures are non-fatal)
    let _guard = logging::init(cli.log_level);

    // Log the parsed CLI invocation
    tracing::debug!(
        "CLI invocation: bs {:?}",
        std::env::args().collect::<Vec<_>>()
    );

    match cli.command {
        // Default command (no subcommand): always detached HEAD.
        // The -b/-B flags require the explicit `bs get` subcommand.
        None => {
            let path = worktree::get_worktree(None)?;
            println!("🌳 {}", path.display());
        }

        // Explicit `bs get` with optional -b/-B branch flags.
        Some(Commands::Get {
            branch,
            new_branch,
            reset_branch,
            tmux_session,
            no_attach,
        }) => {
            // Check up front (before provisioning) so a missing `tmux` fails
            // fast without leaving a half-completed operation behind.
            if tmux_session.is_some() {
                tmux::check_tmux_available()?;
            }

            let branch = match (branch, new_branch, reset_branch) {
                (Some(b), None, None) => Some(worktree::BranchMode::Existing(b)),
                (None, Some(b), None) => Some(worktree::BranchMode::New(b)),
                (None, None, Some(b)) => Some(worktree::BranchMode::Reset(b)),
                _ => None,
            };
            // Capture the name before moving `branch` into get_worktree.
            let branch_name: Option<String> = branch.as_ref().map(|m| match m {
                worktree::BranchMode::New(b)
                | worktree::BranchMode::Reset(b)
                | worktree::BranchMode::Existing(b) => b.clone(),
            });
            let path = worktree::get_worktree(branch)?;
            match branch_name.as_deref() {
                Some(b) => println!("🌳 {}  ({})", path.display(), b),
                None => println!("🌳 {}", path.display()),
            }

            if let Some(value) = tmux_session {
                let repo_name = worktree::repo_slug()?;
                let branch_display = branch_name.as_deref().unwrap_or(tmux::DETACHED_LABEL);
                let session_name = tmux::resolve_session_name(&value, &repo_name, branch_display);
                tmux::ensure_session(&session_name, &path)?;
                println!("\u{1f5a5}\u{fe0f}  tmux session: {}", session_name);
                if !no_attach {
                    tmux::attach_session(&session_name)?;
                }
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

            // Detect the current slot once; tolerate errors (treat as None).
            let current_path: Option<std::path::PathBuf> =
                worktree::current_worktree().ok().flatten().map(|(p, _)| p);

            for (path, status, branch) in &entries {
                let tilde = worktree::tilde_path(path);
                let is_current = current_path.as_deref() == Some(path.as_path());
                let prefix = if is_current { "▶ " } else { "  " };
                let path_display = match branch {
                    Some(b) => format!("{} ({})", tilde, b.bold()),
                    None => tilde,
                };
                // Fixed badge column width, derived from the longest plain badge
                // label ("available" = 9 chars). Update this if a new status
                // variant with a longer label is ever added.
                const BADGE_WIDTH: usize = 9;
                let (plain_badge, colorize): (&str, fn(&str) -> String) = match status {
                    worktree::WorktreeStatus::Locked => {
                        ("locked", |s: &str| s.to_string().yellow().to_string())
                    }
                    worktree::WorktreeStatus::InUse => {
                        ("in use", |s: &str| s.to_string().red().to_string())
                    }
                    worktree::WorktreeStatus::Available => {
                        ("available", |s: &str| s.to_string().green().to_string())
                    }
                };
                let padded_badge = format!("{:<width$}", plain_badge, width = BADGE_WIDTH);
                let badge = colorize(&padded_badge);
                println!("{}{}  {}", prefix, badge, path_display);
            }
        }

        Some(Commands::Status { path }) => {
            let root = worktree::managed_root()?;
            let slug = worktree::repo_slug()?;
            let pool_dir = root.join(&slug);

            let target = match path {
                Some(p) => p,
                None => match worktree::current_worktree()? {
                    Some((p, _)) => p,
                    None => anyhow::bail!(
                        "not inside a managed bonsai pool slot; \
                         please provide a path argument"
                    ),
                },
            };

            worktree::validate_pool_slot(&target, &pool_dir)?;
            let report = worktree::slot_status(&target)?;
            print_status_report(&report);
        }

        Some(Commands::Current) => match worktree::current_worktree()? {
            Some((path, branch)) => {
                let tilde = worktree::tilde_path(&path);
                println!("🌳 {}", format_current_path(&tilde, branch.as_deref()));
            }
            None => {
                println!("Not inside a managed bonsai worktree.");
                println!("Run `bs get` to provision a slot, then `cd` into it.");
                anyhow::bail!("not inside a managed bonsai worktree");
            }
        },

        Some(Commands::Help) => {
            Cli::command().print_long_help()?;
        }

        Some(Commands::Lock { reason, path }) => {
            let root = worktree::managed_root()?;
            let slug = worktree::repo_slug()?;
            let pool_dir = root.join(&slug);

            let target = match path {
                Some(p) => p,
                None => match worktree::current_worktree()? {
                    Some((p, _)) => p,
                    None => anyhow::bail!(
                        "not inside a managed bonsai pool slot; \
                         please provide a path argument"
                    ),
                },
            };

            worktree::validate_pool_slot(&target, &pool_dir)?;
            worktree::lock_worktree(&target, reason.as_deref())?;
            println!("\u{1f512} locked {}", worktree::tilde_path(&target));
        }

        Some(Commands::Unlock { path }) => {
            let root = worktree::managed_root()?;
            let slug = worktree::repo_slug()?;
            let pool_dir = root.join(&slug);

            let target = match path {
                Some(p) => p,
                None => match worktree::current_worktree()? {
                    Some((p, _)) => p,
                    None => anyhow::bail!(
                        "not inside a managed bonsai pool slot; \
                         please provide a path argument"
                    ),
                },
            };

            worktree::validate_pool_slot(&target, &pool_dir)?;
            worktree::unlock_worktree(&target)?;
            println!("\u{1f513} unlocked {}", worktree::tilde_path(&target));
        }

        Some(Commands::Info) => {
            // Gather runtime info
            let version = env!("CARGO_PKG_VERSION");

            // Get the effective log level (from CLI args)
            let log_level = match cli.log_level {
                logging::LogLevel::Trace => "trace",
                logging::LogLevel::Debug => "debug",
                logging::LogLevel::Info => "info",
                logging::LogLevel::Warn => "warn",
                logging::LogLevel::Error => "error",
            };

            // Get log directory and current log file
            let log_dir = logging::log_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to resolve log directory"))?;
            let current_log_file = logging::current_log_file(&log_dir);

            // Get managed root
            let managed_root = worktree::managed_root()?;

            // Format paths with tilde abbreviation
            let log_dir_tilde = worktree::tilde_path(&log_dir);
            let current_log_file_tilde = worktree::tilde_path(&current_log_file);
            let managed_root_tilde = worktree::tilde_path(&managed_root);

            // Print output as key: value lines
            println!("version: {}", version);
            println!("log level: {}", log_level);
            println!("log directory: {}", log_dir_tilde);
            println!("current log file: {}", current_log_file_tilde);
            println!("managed root: {}", managed_root_tilde);
        }

        Some(Commands::Prune) => {
            let root = worktree::managed_root()?;
            let slug = worktree::repo_slug()?;
            let pool_dir = root.join(&slug);

            if !pool_dir.exists() {
                println!("No worktrees managed for this repository (pool does not exist yet).");
                println!("Nothing to prune.");
                return Ok(());
            }

            let outcome = worktree::prune_pool(&pool_dir)?;

            if outcome.pruned.is_empty() && outcome.failures.is_empty() {
                println!("Nothing to prune (no available slots).");
                return Ok(());
            }

            for slot in &outcome.pruned {
                let tilde = worktree::tilde_path(&slot.path);
                println!(
                    "\u{1f5d1}\u{fe0f}  pruned {}",
                    format_current_path(&tilde, slot.branch.as_deref())
                );
            }

            if !outcome.failures.is_empty() {
                for (path, err) in &outcome.failures {
                    let tilde = worktree::tilde_path(path);
                    eprintln!("error: failed to prune {}: {}", tilde, err);
                }
                anyhow::bail!(
                    "failed to prune {} of {} available slot(s)",
                    outcome.failures.len(),
                    outcome.pruned.len() + outcome.failures.len()
                );
            }
        }
    }

    Ok(())
}
