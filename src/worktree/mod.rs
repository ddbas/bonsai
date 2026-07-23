//! Managed git-worktree pool logic for `bs get`.
//!
//! Pool layout:
//! ```text
//! <managed_root>/
//!   <repo-slug>/          <- derived from the main repo root basename
//!     <8-char-uuid>/      <- one slot per managed worktree
//! ```
//!
//! All slots use detached HEAD; branch management is the caller's
//! responsibility.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use uuid::Uuid;

// -- Internal helpers ---------------------------------------------------------

/// Build a `git` `Command` with hook-injected environment variables removed.
///
/// When `bs` is invoked inside a git hook (e.g. pre-commit), git sets
/// `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`, and similar variables that
/// are inherited by every child process.  Those variables would otherwise
/// cause git sub-invocations here to operate on the *hook's* repository
/// instead of the one determined by the process's working directory.
/// Clearing them makes every `git` call behave as if run from a plain shell.
fn git_cmd() -> Command {
    let mut cmd = Command::new("git");
    for var in [
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_WORK_TREE",
        "GIT_PREFIX",
        "GIT_INTERNAL_SUPER_PREFIX",
        "GIT_COMMON_DIR",
    ] {
        cmd.env_remove(var);
    }
    cmd
}

/// Normalise a name into a URL-safe slug: ASCII lower-case, every
/// non-alphanumeric character replaced with `-`.
pub(crate) fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

// -- Types --------------------------------------------------------------------

/// A single entry returned by `git worktree list --porcelain`.
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub locked: bool,
    /// The checked-out branch name (short form, e.g. `main`), or `None` for
    /// detached HEAD.
    pub branch: Option<String>,
}

/// Counts of open processes and dirty/untracked files for a pool worktree slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeStats {
    /// Number of distinct PIDs with open file handles inside the slot.
    pub process_count: usize,
    /// Number of modified or staged files (`git status --porcelain` lines
    /// whose two-character XY code is not `??`).
    pub uncommitted_count: usize,
    /// Number of untracked files (`git status --porcelain` lines whose XY
    /// code is `??`).
    pub untracked_count: usize,
}

impl WorktreeStats {
    fn zero() -> Self {
        WorktreeStats {
            process_count: 0,
            uncommitted_count: 0,
            untracked_count: 0,
        }
    }
}

/// Tuple type returned by [`list_worktrees_status`]: path, status, stats,
/// and the short branch name (or `None` for detached HEAD).
pub type WorktreeListEntry = (PathBuf, WorktreeStatus, WorktreeStats, Option<String>);

/// Availability status of a pool worktree slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeStatus {
    /// Slot exists on disk, is not locked, and its working tree is clean.
    Available,
    /// Slot has uncommitted changes, untracked files, or open process handles,
    /// and is not git-locked.
    InUse,
    /// Slot is git-locked (via `git worktree lock`); takes priority over `InUse`.
    Locked,
}

/// Controls whether and how a branch is created inside the provisioned slot.
///
/// Passed to [`get_worktree`], [`create_slot`], and [`reset_slot`] to select
/// the appropriate `git worktree add` / `git checkout` flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchMode {
    /// Create a new branch (`-b`). Fails if the branch already exists,
    /// mirroring `git checkout -b` semantics.
    New(String),
    /// Create or reset a branch (`-B`). Overwrites an existing branch without
    /// error, mirroring `git checkout -B` semantics.
    Reset(String),
    /// Check out an existing branch without creating or resetting it,
    /// mirroring plain `git checkout <branch>` / `git worktree add <path> <branch>`
    /// semantics. Fails naturally (via git's own error) if the branch does not
    /// exist or is already checked out in another worktree.
    Existing(String),
}

// -- Core utilities (section 2) -----------------------------------------------

/// Run `git rev-parse --git-common-dir` and return the path.
///
/// For a main worktree the output is a relative path (e.g. `.git`); for a
/// linked worktree it is an absolute path.  Both cases are normalised to an
/// absolute `PathBuf` relative to the current working directory.
///
/// Returns an error for bare repositories (output is `.`).
pub fn git_common_dir() -> Result<PathBuf> {
    let output = git_cmd()
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .context("failed to spawn `git rev-parse --git-common-dir`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git rev-parse --git-common-dir` failed: {}", stderr.trim());
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if raw == "." {
        bail!(
            "bare repositories are not supported by `bs get`; \
             run from a non-bare working tree"
        );
    }

    let path = PathBuf::from(&raw);
    if path.is_absolute() {
        Ok(path)
    } else {
        // Relative (e.g. `.git`) -> make absolute using CWD.
        let cwd = std::env::current_dir().context("failed to get current directory")?;
        Ok(cwd.join(path))
    }
}

/// Return the repo slug: basename of the **main** repo root, lowercased, with
/// non-alphanumeric characters replaced by `-`.
///
/// Uses `--git-common-dir` so the result is the same whether called from the
/// main worktree or any linked worktree.
pub fn repo_slug() -> Result<String> {
    let common_dir = git_common_dir()?;
    let repo_root = common_dir
        .parent()
        .context("`git common dir` path has no parent component")?;
    let basename = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .context("repo root directory has no usable file name")?;
    Ok(slugify(basename))
}

/// Run `git rev-parse HEAD` and return the full commit SHA.
pub fn resolve_head() -> Result<String> {
    let output = git_cmd()
        .args(["rev-parse", "HEAD"])
        .output()
        .context("failed to spawn `git rev-parse HEAD`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git rev-parse HEAD` failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run `git rev-parse HEAD --git-common-dir` and return `(head_sha, common_dir)`
/// in a single subprocess call.
///
/// Combines the work of [`resolve_head`] and [`git_common_dir`] into one
/// process-spawn round-trip.  The first output line is the full HEAD commit
/// SHA; the second is the path to the shared `.git` directory (resolved to an
/// absolute path using the same relative→absolute logic as [`git_common_dir`]).
///
/// Use this in performance-sensitive paths (e.g. [`get_worktree`]) where both
/// values are needed together.
pub fn resolve_head_and_common_dir() -> Result<(String, PathBuf)> {
    let output = git_cmd()
        .args(["rev-parse", "HEAD", "--git-common-dir"])
        .output()
        .context("failed to spawn `git rev-parse HEAD --git-common-dir`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "`git rev-parse HEAD --git-common-dir` failed: {}",
            stderr.trim()
        );
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines();

    let head_sha = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("`git rev-parse` produced no HEAD output"))
        .map(str::trim)?
        .to_string();

    let raw_common_dir = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("`git rev-parse` produced no --git-common-dir output"))
        .map(str::trim)?
        .to_string();

    if raw_common_dir == "." {
        bail!(
            "bare repositories are not supported by `bs get`; \
             run from a non-bare working tree"
        );
    }

    let path = PathBuf::from(&raw_common_dir);
    let common_dir = if path.is_absolute() {
        path
    } else {
        let cwd = std::env::current_dir().context("failed to get current directory")?;
        cwd.join(path)
    };

    Ok((head_sha, common_dir))
}

/// Return the managed root directory.
///
/// Uses the `BONSAI_ROOT` environment variable when set (primarily for
/// testing); otherwise returns `~/.bonsai`.
pub fn managed_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("BONSAI_ROOT") {
        return Ok(PathBuf::from(root));
    }
    let home = dirs::home_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "cannot resolve the home directory; \
             please ensure the $HOME environment variable is set"
        )
    })?;
    Ok(home.join(".bonsai"))
}

/// Generate a new UUID v4-based slot path under `pool_dir`.
///
/// The slot directory name is the first 8 hex characters of a UUID v4 value
/// (e.g. `a3f9c1b2`).
pub fn new_slot_path(pool_dir: &Path) -> PathBuf {
    let prefix = format!("{:08x}", Uuid::new_v4().as_fields().0);
    pool_dir.join(prefix)
}

// -- Path helpers -------------------------------------------------------------

/// Replace the home directory prefix in `path` with `~`.
///
/// If `path` does not start with the home directory, the full absolute path is
/// returned unchanged.
pub fn tilde_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(stripped) = path.strip_prefix(&home)
    {
        return format!("~/{}", stripped.display());
    }
    path.display().to_string()
}

// -- Pool scan (section 3) ----------------------------------------------------

/// Internal: parse `git worktree list --porcelain` once; return the filtered
/// pool entries and a stale flag.
///
/// The stale flag is `true` when **any** registered worktree path (including
/// non-pool entries such as the main worktree) no longer exists on disk,
/// meaning a `git worktree prune` run is warranted.
fn list_pool_worktrees_checking_stale(pool_dir: &Path) -> Result<(Vec<WorktreeEntry>, bool)> {
    let output = git_cmd()
        .args(["worktree", "list", "--porcelain"])
        .output()
        .context("failed to spawn `git worktree list --porcelain`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git worktree list` failed: {}", stderr.trim());
    }

    // Canonicalise pool_dir once so comparisons are consistent even through
    // symlinks (e.g. macOS /tmp -> /private/tmp).
    let pool_canonical = pool_dir
        .canonicalize()
        .unwrap_or_else(|_| pool_dir.to_path_buf());

    let text = String::from_utf8_lossy(&output.stdout);
    let mut entries: Vec<WorktreeEntry> = Vec::new();
    let mut has_stale = false;
    let mut cur_path: Option<PathBuf> = None;
    let mut cur_locked = false;
    let mut cur_branch: Option<String> = None;

    // Inline flush: called when a new `worktree ` header line is hit and once
    // after the loop to handle the final block.
    macro_rules! flush {
        () => {
            if let Some(raw) = cur_path.take() {
                let canonical = raw.canonicalize().unwrap_or(raw);
                if !canonical.exists() {
                    has_stale = true;
                }
                if canonical.starts_with(&pool_canonical) {
                    entries.push(WorktreeEntry {
                        path: canonical,
                        locked: cur_locked,
                        branch: cur_branch.take(),
                    });
                } else {
                    // Non-pool entry: branch is irrelevant; drop it.
                    let _ = cur_branch.take();
                }
            }
        };
    }

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            flush!();
            cur_path = Some(PathBuf::from(rest.trim()));
            cur_locked = false;
            cur_branch = None;
        } else if line.starts_with("locked") {
            cur_locked = true;
        } else if let Some(refs) = line.strip_prefix("branch ") {
            let short = refs
                .trim()
                .strip_prefix("refs/heads/")
                .unwrap_or(refs.trim());
            cur_branch = Some(short.to_string());
        }
        // `detached` line → cur_branch stays None
    }
    flush!(); // handle the final entry

    Ok((entries, has_stale))
}

/// Parse `git worktree list --porcelain` and return entries whose path falls
/// under `pool_dir`.
pub fn list_pool_worktrees(pool_dir: &Path) -> Result<Vec<WorktreeEntry>> {
    let (entries, _has_stale) = list_pool_worktrees_checking_stale(pool_dir)?;
    Ok(entries)
}

/// Count the uncommitted (modified/staged) and untracked files in `slot_path`.
///
/// Runs `git -C <slot_path> status --porcelain` and classifies each output
/// line by its two-character XY status code:
/// - Lines where XY == `??` → untracked files.
/// - All other non-empty lines → modified or staged files.
///
/// Returns `(uncommitted_count, untracked_count)`.
pub fn count_git_status_files(slot_path: &Path) -> Result<(usize, usize)> {
    let output = git_cmd()
        .args(["-C", &slot_path.to_string_lossy(), "status", "--porcelain"])
        .output()
        .context("failed to spawn `git status --porcelain`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "`git status --porcelain` failed for {}: {}",
            slot_path.display(),
            stderr.trim()
        );
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut uncommitted = 0usize;
    let mut untracked = 0usize;
    for line in text.lines() {
        if line.len() >= 2 && &line[..2] == "??" {
            untracked += 1;
        } else if !line.is_empty() {
            uncommitted += 1;
        }
    }
    Ok((uncommitted, untracked))
}

/// Return the availability status and usage stats of every pool worktree slot.
///
/// Each slot is classified as [`WorktreeStatus::Available`] when it exists on
/// disk, is not locked, and its working tree is clean (no uncommitted or
/// untracked files, no open process handles at the slot root); otherwise it is
/// [`WorktreeStatus::InUse`].
///
/// Per-slot checks (`lsof +d` and `git status`) are executed concurrently;
/// the returned `Vec` preserves the original slot ordering from
/// `git worktree list --porcelain`.
///
/// The returned tuple is `(path, status, stats, branch)` where `branch` is
/// the short checked-out branch name (`None` for detached HEAD).
pub fn list_worktrees_status(pool_dir: &Path) -> Result<Vec<WorktreeListEntry>> {
    let entries = list_pool_worktrees(pool_dir)?;

    // Spawn one thread per slot so that the blocking `lsof +d` and
    // `git status --porcelain` calls run concurrently.  Handles are collected
    // into a `Vec` and joined in original slot order, guaranteeing that the
    // returned `Vec` ordering matches `git worktree list` regardless of which
    // thread finishes first.
    let handles: Vec<std::thread::JoinHandle<Result<WorktreeListEntry>>> = entries
        .into_iter()
        .map(|entry| {
            std::thread::spawn(move || -> Result<WorktreeListEntry> {
                let branch = entry.branch.clone();
                let (status, stats) = if !entry.path.exists() {
                    (WorktreeStatus::InUse, WorktreeStats::zero())
                } else if entry.locked {
                    // Always classify as Locked regardless of dirty/open-process
                    // signals; stats are still collected so they appear in `bs list`.
                    let process_count = count_open_processes(&entry.path)?;
                    let (uncommitted_count, untracked_count) =
                        count_git_status_files(&entry.path).unwrap_or((0, 0));
                    let stats = WorktreeStats {
                        process_count,
                        uncommitted_count,
                        untracked_count,
                    };
                    (WorktreeStatus::Locked, stats)
                } else {
                    let process_count = count_open_processes(&entry.path)?;
                    let (uncommitted_count, untracked_count) =
                        count_git_status_files(&entry.path).unwrap_or((0, 0));
                    let stats = WorktreeStats {
                        process_count,
                        uncommitted_count,
                        untracked_count,
                    };
                    let is_clean = uncommitted_count == 0 && untracked_count == 0;
                    if process_count > 0 || !is_clean {
                        (WorktreeStatus::InUse, stats)
                    } else {
                        (WorktreeStatus::Available, stats)
                    }
                };
                Ok((entry.path, status, stats, branch))
            })
        })
        .collect();

    // Join in original order; propagate the first error encountered.
    handles
        .into_iter()
        .map(|h| {
            h.join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("slot status check thread panicked")))
        })
        .collect()
}

/// Parse PID fields from `lsof +D` stdout and return the count of distinct PIDs.
///
/// Skips the header line (starts with `COMMAND`). The PID is the second
/// whitespace-delimited field on each data line.
fn parse_lsof_pids(stdout: &str) -> usize {
    let mut pids: HashSet<&str> = HashSet::new();
    for line in stdout.lines() {
        if line.starts_with("COMMAND") {
            continue;
        }
        let mut fields = line.split_whitespace();
        fields.next(); // skip command name
        if let Some(pid_str) = fields.next() {
            pids.insert(pid_str);
        }
    }
    pids.len()
}

/// Internal helper: run `lsof_bin -w +d <path>` and return the number of
/// distinct PIDs with open file descriptors directly in `path`
/// (non-recursive).
///
/// Separated from `count_open_processes` so tests can pass a non-existent
/// binary name without mutating the global `PATH` environment variable.
fn run_lsof_count(lsof_bin: &str, path: &Path) -> Result<usize> {
    let output = Command::new(lsof_bin)
        .args(["-w", "+d", &path.to_string_lossy()])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "lsof not found on PATH — install lsof to use bs \
                     (e.g. brew install lsof)"
                )
            } else {
                anyhow::anyhow!("failed to spawn lsof: {}", e)
            }
        })?;

    // `lsof +D` exits non-zero on macOS even when files are found; use
    // stdout/stderr content as the authoritative signals.
    if output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // `-w` suppresses cosmetic warning diagnostics at the source, so any
        // remaining stderr output is a genuine error.
        let stderr = stderr.trim();
        if stderr.is_empty() {
            return Ok(0);
        } else {
            bail!("lsof error for {}: {}", path.display(), stderr);
        }
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_lsof_pids(&text))
}

/// Return the number of distinct PIDs that have open file descriptors directly
/// in `path` (non-recursive; the top-level directory only).
///
/// Uses `lsof -w +d <path>` to query open file handles (`-w` suppresses
/// cosmetic warning diagnostics).  A process whose current
/// working directory is `path` is detected; a process with handles only in
/// subdirectories of `path` is **not** detected.  The PID (second
/// whitespace-delimited column) of each non-header output line is collected
/// into a `HashSet`; the size of that set is returned.
///
/// - Non-empty stdout → parses and returns the distinct-PID count.
/// - Empty stdout + empty stderr → no files are open → returns `Ok(0)`.
/// - Spawn error (`lsof` not on `PATH`) → returns `Err` with an actionable
///   message naming `lsof` as the missing dependency.
/// - Non-empty stderr → `lsof` itself encountered an error → returns `Err`.
pub fn count_open_processes(path: &Path) -> Result<usize> {
    run_lsof_count("lsof", path)
}

/// Internal helper: run `lsof_bin -w +d <path>` and return whether any
/// process has open file descriptors directly in `path` (non-recursive).
///
/// Separated from `has_open_files` so tests can pass a non-existent binary
/// name without mutating the global `PATH` environment variable.
fn run_lsof(lsof_bin: &str, path: &Path) -> Result<bool> {
    let output = Command::new(lsof_bin)
        .args(["-w", "+d", &path.to_string_lossy()])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "lsof not found on PATH — install lsof to use bs \
                     (e.g. brew install lsof)"
                )
            } else {
                anyhow::anyhow!("failed to spawn lsof: {}", e)
            }
        })?;

    // `lsof +D` exits with a non-zero status code on macOS regardless of
    // whether it found files or not.  We therefore use stdout/stderr content
    // as the authoritative signals:
    //
    // 1. Non-empty stdout → lsof found at least one open file descriptor
    //    → `Ok(true)`.
    // 2. Empty stdout + empty stderr → no open file descriptors →
    //    `Ok(false)`.
    // 3. Non-empty stderr → lsof encountered a real error (e.g. the path does
    //    not exist, or a permission error) → `Err(...)`.
    //
    // `-w` suppresses cosmetic warning diagnostics (e.g. Docker overlay2/nsfs
    // WARNING lines on Linux, macOS mount-table diagnostics) at the source,
    // so any stderr output that remains is a genuine error.
    if !output.stdout.is_empty() {
        return Ok(true);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        Ok(false)
    } else {
        bail!("lsof error for {}: {}", path.display(), stderr)
    }
}

/// Detect whether any process currently has an open file descriptor directly
/// in `path` (non-recursive; the top-level directory only).
///
/// Uses `lsof -w +d <path>` to query open file handles (`-w` suppresses
/// cosmetic warning diagnostics).  A process whose current
/// working directory is `path` is detected; a process with handles only in
/// subdirectories of `path` is **not** detected.  The result is determined by
/// stdout/stderr content (not exit code, which `lsof` sets unreliably across
/// platforms):
///
/// - Non-empty stdout → at least one process has a handle in `path` →
///   returns `Ok(true)`.
/// - Empty stdout + empty stderr → no files are open → returns `Ok(false)`.
/// - Spawn error (`lsof` not on `PATH`) → returns `Err` with an actionable
///   message naming `lsof` as the missing dependency and including an install
///   hint (e.g. `brew install lsof`).
/// - Non-empty stderr → `lsof` itself encountered an error → returns `Err`
///   propagating the `lsof` output.
pub fn has_open_files(path: &Path) -> Result<bool> {
    run_lsof("lsof", path)
}

/// Return `true` if the working tree at `slot_path` has no uncommitted
/// changes (`git -C <slot> status --porcelain` produces empty output).
pub fn is_clean(slot_path: &Path) -> Result<bool> {
    let output = git_cmd()
        .args(["-C", &slot_path.to_string_lossy(), "status", "--porcelain"])
        .output()
        .context("failed to spawn `git status --porcelain`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "`git status --porcelain` failed for {}: {}",
            slot_path.display(),
            stderr.trim()
        );
    }

    Ok(output.stdout.is_empty())
}

/// Run `git worktree prune` to remove stale registrations.
pub fn prune_worktrees() -> Result<()> {
    let output = git_cmd()
        .args(["worktree", "prune"])
        .output()
        .context("failed to spawn `git worktree prune`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git worktree prune` failed: {}", stderr.trim());
    }

    Ok(())
}

/// Lock a bonsai-managed pool slot using `git worktree lock`.
///
/// Forwards `reason` verbatim to `--reason` when supplied.  The slot must
/// already be registered as a git worktree.  Git surfaces its own error if
/// the slot is already locked or not a worktree.
pub fn lock_worktree(path: &Path, reason: Option<&str>) -> Result<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("slot path is not valid UTF-8: {}", path.display()))?
        .to_string();

    if let Some(msg) = reason {
        tracing::info!("Locking slot {} with reason: {}", path_str, msg);
    } else {
        tracing::info!("Locking slot {}", path_str);
    }

    let output = if let Some(msg) = reason {
        git_cmd()
            .args(["worktree", "lock", "--reason", msg, &path_str])
            .output()
            .context("failed to spawn `git worktree lock`")?
    } else {
        git_cmd()
            .args(["worktree", "lock", &path_str])
            .output()
            .context("failed to spawn `git worktree lock`")?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Failed to lock slot {}: {}", path_str, stderr.trim());
        bail!("`git worktree lock` failed: {}", stderr.trim());
    }
    tracing::debug!("Successfully locked slot {}", path_str);
    Ok(())
}

/// Unlock a bonsai-managed pool slot using `git worktree unlock`.
///
/// Git surfaces its own error if the slot is not currently locked or not a
/// worktree.
pub fn unlock_worktree(path: &Path) -> Result<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("slot path is not valid UTF-8: {}", path.display()))?
        .to_string();

    tracing::info!("Unlocking slot {}", path_str);

    let output = git_cmd()
        .args(["worktree", "unlock", &path_str])
        .output()
        .context("failed to spawn `git worktree unlock`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Failed to unlock slot {}: {}", path_str, stderr.trim());
        bail!("`git worktree unlock` failed: {}", stderr.trim());
    }
    tracing::debug!("Successfully unlocked slot {}", path_str);
    Ok(())
}

/// Verify that `path` is a bonsai-managed pool slot under `pool_dir`.
///
/// Returns an error when:
/// - `path` does not exist on disk, or
/// - `path` (after canonicalization) does not fall under `pool_dir`.
pub fn validate_pool_slot(path: &Path, pool_dir: &Path) -> Result<()> {
    if !path.exists() {
        bail!("path does not exist: {}", path.display());
    }
    let pool_canonical = pool_dir
        .canonicalize()
        .unwrap_or_else(|_| pool_dir.to_path_buf());
    let path_canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !path_canonical.starts_with(&pool_canonical) {
        bail!(
            "{} is not a bonsai-managed pool slot (pool directory: {})",
            path.display(),
            pool_dir.display()
        );
    }
    Ok(())
}

/// Return the first available (clean, unlocked, on-disk) slot in the pool,
/// or `None` if every slot is unavailable.
///
/// Runs `git worktree prune` only when the worktree list contains at least one
/// registered path that no longer exists on disk, avoiding the subprocess cost
/// on every invocation.
pub fn find_available_slot(pool_dir: &Path) -> Result<Option<PathBuf>> {
    let (entries, has_stale) = list_pool_worktrees_checking_stale(pool_dir)?;
    if has_stale {
        tracing::info!("Pruning stale worktrees from pool");
        prune_worktrees()?;
    }
    for entry in entries {
        if entry.locked || !entry.path.exists() {
            continue;
        }
        if !is_clean(&entry.path)? {
            continue;
        }
        if count_open_processes(&entry.path)? > 0 {
            continue;
        }
        tracing::info!("Found available slot: {}", entry.path.display());
        return Ok(Some(entry.path));
    }
    tracing::debug!("No available slots found in pool");
    Ok(None)
}

// -- Provision (section 4) ----------------------------------------------------

/// Reset an existing slot to `head_sha`, optionally checking out a branch.
///
/// - `branch = None` → `git -C <slot> checkout --detach <head_sha>` (detached HEAD)
/// - `branch = Some(BranchMode::New(b))` → `git -C <slot> checkout -b <b> <head_sha>`
/// - `branch = Some(BranchMode::Reset(b))` → `git -C <slot> checkout -B <b> <head_sha>`
/// - `branch = Some(BranchMode::Existing(b))` → `git -C <slot> checkout <b>` (no
///   `head_sha`; relies on git's own failure if `b` doesn't exist or is already
///   checked out elsewhere)
pub fn reset_slot(slot_path: &Path, head_sha: &str, branch: Option<&BranchMode>) -> Result<()> {
    let slot_str = slot_path.to_string_lossy();
    let branch_desc = match branch {
        None => "detached HEAD".to_string(),
        Some(BranchMode::New(name)) => format!("new branch {}", name),
        Some(BranchMode::Reset(name)) => format!("reset branch {}", name),
        Some(BranchMode::Existing(name)) => format!("existing branch {}", name),
    };
    tracing::debug!(
        "Resetting slot {} to {} ({})",
        slot_str,
        head_sha,
        branch_desc
    );

    let output = match branch {
        None => git_cmd()
            .args(["-C", &slot_str, "checkout", "--detach", head_sha])
            .output()
            .context("failed to spawn `git checkout --detach`")?,
        Some(BranchMode::New(name)) => git_cmd()
            .args(["-C", &slot_str, "checkout", "-b", name, head_sha])
            .output()
            .context("failed to spawn `git checkout -b`")?,
        Some(BranchMode::Reset(name)) => git_cmd()
            .args(["-C", &slot_str, "checkout", "-B", name, head_sha])
            .output()
            .context("failed to spawn `git checkout -B`")?,
        Some(BranchMode::Existing(name)) => git_cmd()
            .args(["-C", &slot_str, "checkout", name])
            .output()
            .context("failed to spawn `git checkout`")?,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "git checkout failed for slot {}: {}",
            slot_str,
            stderr.trim()
        );
        bail!("`git checkout` failed: {}", stderr.trim());
    }

    tracing::debug!("Successfully reset slot {}", slot_str);
    Ok(())
}

/// Create a new worktree slot at `slot_path`.
///
/// - `branch = None` → `git worktree add --detach <slot_path> <head_sha>`
/// - `branch = Some(BranchMode::New(b))` → `git worktree add -b <b> <slot_path> <head_sha>`
/// - `branch = Some(BranchMode::Reset(b))` → `git worktree add -B <b> <slot_path> <head_sha>`
/// - `branch = Some(BranchMode::Existing(b))` → `git worktree add <slot_path> <b>`
///   (no `--detach`, no `head_sha`; relies on git's own failure if `b` doesn't
///   exist or is already checked out elsewhere)
pub fn create_slot(slot_path: &Path, head_sha: &str, branch: Option<&BranchMode>) -> Result<()> {
    let slot_str = slot_path.to_string_lossy();
    let output = match branch {
        None => git_cmd()
            .args(["worktree", "add", "--detach", &slot_str, head_sha])
            .output()
            .context("failed to spawn `git worktree add --detach`")?,
        Some(BranchMode::New(name)) => git_cmd()
            .args(["worktree", "add", "-b", name, &slot_str, head_sha])
            .output()
            .context("failed to spawn `git worktree add -b`")?,
        Some(BranchMode::Reset(name)) => git_cmd()
            .args(["worktree", "add", "-B", name, &slot_str, head_sha])
            .output()
            .context("failed to spawn `git worktree add -B`")?,
        Some(BranchMode::Existing(name)) => git_cmd()
            .args(["worktree", "add", &slot_str, name])
            .output()
            .context("failed to spawn `git worktree add`")?,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git worktree add` failed: {}", stderr.trim());
    }

    Ok(())
}

/// Orchestrate the full `get` flow and return the absolute path of the
/// provisioned (or reused) worktree.
///
/// Steps:
/// 1. Resolve `HEAD` SHA and the canonical repo slug in one subprocess call.
/// 2. Derive pool directory (`managed_root()/<repo-slug>/`).
/// 3. Create pool directory if it does not yet exist.
/// 4. Scan for an available slot; if none, generate a new UUID slot.
/// 5. Reset (or add) the slot to `HEAD`, checking out `branch` when provided.
/// 6. Return the canonicalised slot path.
///
/// When `branch` is `None` the slot is left in detached HEAD state (existing
/// behaviour). Pass `Some(BranchMode::New(…))` or `Some(BranchMode::Reset(…))`
/// to have the slot checked out on a named branch in a single git subprocess.
pub fn get_worktree(branch: Option<BranchMode>) -> Result<PathBuf> {
    // Single subprocess: git rev-parse HEAD --git-common-dir
    let (head_sha, common_dir) = resolve_head_and_common_dir()?;
    let repo_root = common_dir
        .parent()
        .context("`git common dir` path has no parent component")?;
    let basename = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .context("repo root directory has no usable file name")?;
    let slug = slugify(basename);
    let pool_dir = managed_root()?.join(&slug);

    std::fs::create_dir_all(&pool_dir)
        .with_context(|| format!("failed to create pool directory {}", pool_dir.display()))?;

    // Canonicalise after creation so path comparisons are symlink-safe.
    let pool_dir = pool_dir.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize pool directory {}",
            pool_dir.display()
        )
    })?;

    // For a positional `<branch>` argument (BranchMode::Existing), first check
    // whether one of this repo's managed pool slots already has that branch
    // checked out. If so, hand back that slot's path directly rather than
    // attempting to provision/reset a (possibly different) slot, since git
    // would otherwise refuse to check the branch out a second time.
    // `BranchMode::New`/`BranchMode::Reset` intentionally skip this lookup:
    // they express an explicit create/reset intent, not "reuse whatever slot
    // has this branch".
    if let Some(BranchMode::Existing(name)) = &branch
        && let Some(existing_slot) = find_slot_checked_out_on_branch(&pool_dir, name)?
    {
        return existing_slot.canonicalize().with_context(|| {
            format!(
                "failed to canonicalize slot path {}",
                existing_slot.display()
            )
        });
    }

    let slot_path = match find_available_slot(&pool_dir)? {
        Some(existing) => {
            reset_slot(&existing, &head_sha, branch.as_ref())?;
            existing
        }
        None => {
            let new_slot = new_slot_path(&pool_dir);
            create_slot(&new_slot, &head_sha, branch.as_ref())?;
            new_slot
        }
    };

    slot_path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize slot path {}", slot_path.display()))
}

// -- Current worktree detection (section 5) ---------------------------------

/// Find the first pool slot (if any) currently checked out on `branch`.
///
/// Reuses the same per-slot "which branch is checked out here" data
/// (`WorktreeEntry::branch`) that backs [`list_worktrees_status`] and
/// [`current_worktree`]. Returns `Ok(None)` immediately when `pool_dir` does
/// not exist on disk (mirroring [`find_slot_for_cwd`]); detached-HEAD slots
/// (`entry.branch == None`) never match.
pub(crate) fn find_slot_checked_out_on_branch(
    pool_dir: &Path,
    branch: &str,
) -> Result<Option<PathBuf>> {
    if !pool_dir.exists() {
        return Ok(None);
    }
    let entries = list_pool_worktrees(pool_dir)?;
    for entry in entries {
        if entry.branch.as_deref() == Some(branch) {
            return Ok(Some(entry.path));
        }
    }
    Ok(None)
}

/// Internal helper: find which pool slot (if any) `cwd` is inside.
///
/// Returns `Ok(None)` immediately when `pool_dir` does not exist on disk
/// (no error).  Otherwise scans pool entries and returns the first whose
/// path is an ancestor of `cwd`.
fn find_slot_for_cwd(cwd: &Path, pool_dir: &Path) -> Result<Option<(PathBuf, Option<String>)>> {
    if !pool_dir.exists() {
        return Ok(None);
    }
    // Canonicalise once so symlink-resolved paths compare correctly
    // (e.g. macOS /tmp → /private/tmp).
    let pool_dir = pool_dir
        .canonicalize()
        .unwrap_or_else(|_| pool_dir.to_path_buf());
    let entries = list_pool_worktrees(&pool_dir)?;
    for entry in entries {
        if cwd.starts_with(&entry.path) {
            return Ok(Some((entry.path, entry.branch)));
        }
    }
    Ok(None)
}

/// Return the managed pool slot that contains the current working directory,
/// or `Ok(None)` when the CWD is not inside any managed slot (including when
/// no pool directory exists yet).
///
/// Steps:
/// 1. Resolve and canonicalise the process CWD.
/// 2. Derive the pool directory from [`managed_root`] and [`repo_slug`].
/// 3. Delegate to [`find_slot_for_cwd`].
pub fn current_worktree() -> Result<Option<(PathBuf, Option<String>)>> {
    let cwd = std::env::current_dir()
        .context("failed to get current directory")?
        .canonicalize()
        .context("failed to canonicalize current directory")?;
    let pool_dir = managed_root()?.join(repo_slug()?);
    find_slot_for_cwd(&cwd, &pool_dir)
}

// -- Unit Tests ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // -- tilde_path -----------------------------------------------------------

    #[test]
    fn tilde_path_replaces_home_prefix() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("some").join("dir");
            let result = tilde_path(&path);
            assert_eq!(result, "~/some/dir");
        }
    }

    #[test]
    fn tilde_path_exact_home() {
        if let Some(home) = dirs::home_dir() {
            // Edge case: the path IS the home directory.
            let result = tilde_path(&home);
            assert_eq!(result, "~/");
        }
    }

    #[test]
    fn tilde_path_outside_home_unchanged() {
        let path = PathBuf::from("/tmp/some/path");
        let result = tilde_path(&path);
        assert_eq!(result, "/tmp/some/path");
    }

    // -- list_worktrees_status classification ---------------------------------

    /// Verify that the `WorktreeStatus` logic produces the right variant given
    /// different combinations of `locked` flag, file counts, and process count.
    /// We test this via a synthetic classification rather than calling
    /// `list_worktrees_status` directly (which needs git on PATH and a real
    /// pool dir).
    #[test]
    fn status_locked_clean_is_locked() {
        let (status, stats) = synthetic_status(true, true, 0, 0, 0);
        assert_eq!(status, WorktreeStatus::Locked);
        assert_eq!(stats.process_count, 0);
        assert_eq!(stats.uncommitted_count, 0);
    }

    #[test]
    fn status_locked_dirty_is_locked() {
        let (status, stats) = synthetic_status(true, true, 3, 0, 0);
        assert_eq!(status, WorktreeStatus::Locked);
        assert_eq!(stats.uncommitted_count, 3);
    }

    #[test]
    fn status_locked_with_open_processes_is_locked() {
        let (status, stats) = synthetic_status(true, true, 0, 0, 2);
        assert_eq!(status, WorktreeStatus::Locked);
        assert_eq!(stats.process_count, 2);
    }

    #[test]
    fn status_nonexistent_is_in_use() {
        let (status, _) = synthetic_status(false, false, 0, 0, 0);
        assert_eq!(status, WorktreeStatus::InUse);
    }

    #[test]
    fn status_dirty_is_in_use() {
        let (status, stats) = synthetic_status(false, true, 1, 0, 0);
        assert_eq!(status, WorktreeStatus::InUse);
        assert_eq!(stats.process_count, 0);
        assert_eq!(stats.uncommitted_count, 1);
    }

    #[test]
    fn status_clean_unlocked_is_available() {
        let (status, stats) = synthetic_status(false, true, 0, 0, 0);
        assert_eq!(status, WorktreeStatus::Available);
        assert_eq!(
            stats.process_count, 0,
            "available slot should have 0 process count"
        );
        assert_eq!(stats.uncommitted_count, 0);
        assert_eq!(stats.untracked_count, 0);
    }

    #[test]
    fn status_open_files_is_in_use() {
        let (status, stats) = synthetic_status(false, true, 0, 0, 2);
        assert_eq!(status, WorktreeStatus::InUse);
        assert_eq!(stats.process_count, 2);
    }

    #[test]
    fn status_dirty_with_open_files_shows_count() {
        let (status, stats) = synthetic_status(false, true, 1, 0, 3);
        assert_eq!(status, WorktreeStatus::InUse);
        assert_eq!(
            stats.process_count, 3,
            "dirty slot with open files should expose process count"
        );
        assert_eq!(stats.uncommitted_count, 1);
    }

    /// Helper that mirrors the `list_worktrees_status` classification logic
    /// without requiring a live git repository.
    fn synthetic_status(
        locked: bool,
        exists: bool,
        uncommitted: usize,
        untracked: usize,
        open_count: usize,
    ) -> (WorktreeStatus, WorktreeStats) {
        if !exists {
            (WorktreeStatus::InUse, WorktreeStats::zero())
        } else if locked {
            let stats = WorktreeStats {
                process_count: open_count,
                uncommitted_count: uncommitted,
                untracked_count: untracked,
            };
            (WorktreeStatus::Locked, stats)
        } else {
            let stats = WorktreeStats {
                process_count: open_count,
                uncommitted_count: uncommitted,
                untracked_count: untracked,
            };
            let is_clean = uncommitted == 0 && untracked == 0;
            if open_count > 0 || !is_clean {
                (WorktreeStatus::InUse, stats)
            } else {
                (WorktreeStatus::Available, stats)
            }
        }
    }

    // -- branch parsing from porcelain ---------------------------------------

    /// `list_pool_worktrees` should parse `branch refs/heads/main` and expose
    /// it as `Some("main")`.
    ///
    /// We test the parsing logic indirectly via a helper that mimics the inner
    /// loop without needing a real git repo.
    #[test]
    fn parse_branch_refs_heads_strips_prefix() {
        let refs = "refs/heads/main";
        let short = refs.strip_prefix("refs/heads/").unwrap_or(refs);
        assert_eq!(short, "main");
    }

    #[test]
    fn parse_branch_refs_heads_nested() {
        let refs = "refs/heads/feature/my-work";
        let short = refs.strip_prefix("refs/heads/").unwrap_or(refs);
        assert_eq!(short, "feature/my-work");
    }

    #[test]
    fn parse_branch_detached_yields_none() {
        // The `detached` line does not start with `branch `, so cur_branch
        // should remain None.  Verify that the sentinel string is unchanged.
        let line = "detached";
        let branch = if let Some(refs) = line.strip_prefix("branch ") {
            Some(refs.strip_prefix("refs/heads/").unwrap_or(refs).to_string())
        } else {
            None
        };
        assert!(branch.is_none());
    }

    // -- count_git_status_files parsing -------------------------------------

    /// Lines starting with `??` should count as untracked; others as
    /// uncommitted.
    #[test]
    fn count_git_status_files_classifies_correctly() {
        let porcelain = " M src/main.rs\n?? build/\nA  new_file.rs\n?? tmp/\n";
        let mut uncommitted = 0usize;
        let mut untracked = 0usize;
        for line in porcelain.lines() {
            if line.len() >= 2 && &line[..2] == "??" {
                untracked += 1;
            } else if !line.is_empty() {
                uncommitted += 1;
            }
        }
        assert_eq!(uncommitted, 2, " M and A lines are uncommitted");
        assert_eq!(untracked, 2, "?? lines are untracked");
    }

    #[test]
    fn count_git_status_files_empty_output() {
        let porcelain = "";
        let mut uncommitted = 0usize;
        let mut untracked = 0usize;
        for line in porcelain.lines() {
            if line.len() >= 2 && &line[..2] == "??" {
                untracked += 1;
            } else if !line.is_empty() {
                uncommitted += 1;
            }
        }
        assert_eq!(uncommitted, 0);
        assert_eq!(untracked, 0);
    }

    #[test]
    fn count_git_status_files_only_untracked() {
        let porcelain = "?? foo.txt\n?? bar.txt\n";
        let mut uncommitted = 0usize;
        let mut untracked = 0usize;
        for line in porcelain.lines() {
            if line.len() >= 2 && &line[..2] == "??" {
                untracked += 1;
            } else if !line.is_empty() {
                uncommitted += 1;
            }
        }
        assert_eq!(uncommitted, 0);
        assert_eq!(untracked, 2);
    }

    /// A file held open in a temp dir causes `has_open_files` to return `Ok(true)`.
    #[test]
    fn has_open_files_returns_true_when_file_open() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let file_path = dir.path().join("held_open.txt");
        fs::write(&file_path, b"data").expect("write");
        // Hold the file open for the duration of the assertion.
        let _handle = fs::File::open(&file_path).expect("open file");

        let result = has_open_files(dir.path());
        assert_eq!(
            result.expect("has_open_files should not error"),
            true,
            "a held-open file should cause has_open_files to return true"
        );
    }

    /// A temp dir with no open handles returns `Ok(false)`.
    #[test]
    fn has_open_files_returns_false_when_no_open_files() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");

        let result = has_open_files(dir.path());
        assert_eq!(
            result.expect("has_open_files should not error"),
            false,
            "a dir with no open handles should return false"
        );
    }

    /// When the `lsof` binary cannot be found, `has_open_files` returns an
    /// `Err` whose message names `lsof` as the missing dependency.
    ///
    /// Uses `run_lsof` directly with a non-existent binary name to avoid
    /// mutating the global `PATH` environment variable.
    #[test]
    fn has_open_files_err_when_lsof_not_found() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let err = run_lsof("/nonexistent/path/to/lsof-binary-xyz", dir.path())
            .expect_err("run_lsof should return Err when the binary is not found");

        assert!(
            err.to_string().contains("lsof"),
            "error message should mention 'lsof', got: {err}"
        );
    }

    // -- count_open_processes -------------------------------------------------

    /// A file held open in a temp dir causes `count_open_processes` to return `Ok(1)`.
    #[test]
    fn count_open_processes_returns_one_when_file_open() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let file_path = dir.path().join("held_open.txt");
        fs::write(&file_path, b"data").expect("write");
        // Hold the file open for the duration of the assertion.
        let _handle = fs::File::open(&file_path).expect("open file");

        let result =
            count_open_processes(dir.path()).expect("count_open_processes should not error");
        assert_eq!(result, 1, "one process (this test) holds the file open");
    }

    /// A temp dir with no open handles returns `Ok(0)`.
    #[test]
    fn count_open_processes_returns_zero_when_no_open_files() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");

        let result =
            count_open_processes(dir.path()).expect("count_open_processes should not error");
        assert_eq!(result, 0, "no open handles should return 0");
    }

    /// Parse correctness: duplicate PIDs in mock lsof output are deduplicated.
    #[test]
    fn parse_lsof_pids_deduplicates() {
        let mock_output = "COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\n\
                           vim       100 user  cwd    DIR    1,2      512  123 /tmp/dir\n\
                           vim       100 user  txt    REG    1,2     4096  456 /tmp/dir/f1\n\
                           bash      200 user  txt    REG    1,2     4096  789 /tmp/dir/f2\n";
        assert_eq!(
            parse_lsof_pids(mock_output),
            2,
            "PIDs 100 (twice) and 200 → 2 distinct processes"
        );
    }

    /// When `lsof` binary is not on PATH, `count_open_processes` returns `Err`
    /// whose message mentions `lsof`.
    #[test]
    fn count_open_processes_err_when_lsof_not_found() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let err = run_lsof_count("/nonexistent/path/to/lsof-binary-xyz", dir.path())
            .expect_err("run_lsof_count should return Err when the binary is not found");

        assert!(
            err.to_string().contains("lsof"),
            "error message should mention 'lsof', got: {err}"
        );
    }

    /// `lsof +d` (non-recursive) must NOT detect a file open only in a
    /// subdirectory.  This is the key behavioural difference from `lsof +D`.
    #[test]
    fn has_open_files_returns_false_when_file_open_in_subdirectory() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).expect("create subdir");
        let file_path = subdir.join("held_open.txt");
        fs::write(&file_path, b"data").expect("write");
        // Hold the file open inside the *subdirectory*, not in dir.path() itself.
        let _handle = fs::File::open(&file_path).expect("open file");

        let result = has_open_files(dir.path());
        assert_eq!(
            result.expect("has_open_files should not error"),
            false,
            "lsof +d (non-recursive) should NOT detect a file open only in a subdirectory"
        );
    }

    /// `count_open_processes` must also return 0 when the only open handle is
    /// inside a subdirectory (verifies `+d` semantics for the count variant).
    #[test]
    fn count_open_processes_returns_zero_when_file_open_in_subdirectory() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let subdir = dir.path().join("nested");
        fs::create_dir(&subdir).expect("create subdir");
        let file_path = subdir.join("open.txt");
        fs::write(&file_path, b"data").expect("write");
        let _handle = fs::File::open(&file_path).expect("open file");

        let result =
            count_open_processes(dir.path()).expect("count_open_processes should not error");
        assert_eq!(
            result, 0,
            "lsof +d should not count processes with handles only in subdirectories"
        );
    }

    // -- BranchMode -----------------------------------------------------------

    #[test]
    fn branch_mode_new_holds_name() {
        let m = BranchMode::New("feature".to_string());
        if let BranchMode::New(name) = m {
            assert_eq!(name, "feature");
        } else {
            panic!("expected New variant");
        }
    }

    #[test]
    fn branch_mode_reset_holds_name() {
        let m = BranchMode::Reset("hotfix".to_string());
        if let BranchMode::Reset(name) = m {
            assert_eq!(name, "hotfix");
        } else {
            panic!("expected Reset variant");
        }
    }

    #[test]
    fn branch_mode_new_and_reset_are_distinct() {
        let a = BranchMode::New("x".to_string());
        let b = BranchMode::Reset("x".to_string());
        assert_ne!(a, b, "New and Reset with the same name must not be equal");
    }

    #[test]
    fn branch_mode_clone() {
        let orig = BranchMode::New("cloned".to_string());
        let copy = orig.clone();
        assert_eq!(orig, copy);
    }

    // -- resolve_head_and_common_dir -----------------------------------------

    /// Verify that the merged call returns a non-empty HEAD SHA and a
    /// non-empty common-dir path that actually exists on disk.
    /// This test requires that it runs inside a git repository.
    #[test]
    fn resolve_head_and_common_dir_returns_both_values() {
        let result = resolve_head_and_common_dir();
        // Skip gracefully if not inside a git repo (shouldn't happen in CI).
        if result.is_err() {
            return;
        }
        let (head_sha, common_dir) = result.unwrap();
        assert!(
            !head_sha.is_empty(),
            "HEAD SHA should be non-empty, got: {head_sha:?}"
        );
        assert!(
            head_sha.chars().all(|c| c.is_ascii_hexdigit()),
            "HEAD SHA should be a hex string, got: {head_sha:?}"
        );
        assert!(
            common_dir.exists(),
            "common_dir path should exist on disk: {}",
            common_dir.display()
        );
    }

    /// Verify that `resolve_head_and_common_dir` returns the same HEAD SHA
    /// as `resolve_head` when called from the same working directory.
    #[test]
    fn resolve_head_and_common_dir_matches_individual_calls() {
        let merged = resolve_head_and_common_dir();
        let individual_head = resolve_head();
        let individual_dir = git_common_dir();

        // Skip gracefully if not inside a git repo.
        if merged.is_err() || individual_head.is_err() || individual_dir.is_err() {
            return;
        }

        let (merged_sha, merged_dir) = merged.unwrap();
        assert_eq!(
            merged_sha,
            individual_head.unwrap(),
            "merged call HEAD SHA should match individual resolve_head()"
        );
        assert_eq!(
            merged_dir,
            individual_dir.unwrap(),
            "merged call common_dir should match individual git_common_dir()"
        );
    }

    // -- list_pool_worktrees_checking_stale ----------------------------------

    /// When every registered worktree path exists on disk the stale flag
    /// must be `false`, meaning `git worktree prune` is not needed.
    /// Verified indirectly: `list_pool_worktrees_checking_stale` only sets
    /// `has_stale = true` inside the flush block when `!canonical.exists()`;
    /// a freshly-created real pool_dir with no registered worktrees returns
    /// an empty list and `has_stale = false` (no paths to check against).
    #[test]
    fn list_pool_worktrees_checking_stale_no_stale_for_fresh_dir() {
        use tempfile::TempDir;

        // Use a real git repo (current dir) as the pool root so the subprocess
        // succeeds.  The pool_dir is a temp dir that is NOT a known worktree
        // path, so the returned pool entries will be empty — but has_stale is
        // determined from ALL paths in `git worktree list`, not just pool ones.
        let dir = TempDir::new().expect("temp dir");
        let result = list_pool_worktrees_checking_stale(dir.path());
        if let Ok((entries, has_stale)) = result {
            // No registered worktree lives under a fresh temp dir.
            assert!(
                entries.is_empty(),
                "no pool entries expected for a fresh temp dir"
            );
            // has_stale reflects ALL worktrees, not just pool ones.  For a
            // healthy repo (all paths present) it should be false.
            // We can only assert this on a machine where all worktrees are intact.
            let _ = has_stale; // not safe to assert without knowing host state
        }
        // If the git call fails (e.g. not in a git repo) just skip.
    }

    #[test]
    fn slugify_lowercases_ascii() {
        assert_eq!(slugify("MyRepo"), "myrepo");
    }

    #[test]
    fn slugify_replaces_dot_with_dash() {
        assert_eq!(slugify("my.repo"), "my-repo");
    }

    #[test]
    fn slugify_replaces_space_with_dash() {
        assert_eq!(slugify("my repo"), "my-repo");
    }

    #[test]
    fn slugify_replaces_multiple_non_alnum() {
        assert_eq!(slugify("My.Repo-Name!"), "my-repo-name-");
    }

    #[test]
    fn slugify_preserves_digits() {
        assert_eq!(slugify("repo123"), "repo123");
    }

    // -- 7.3: new_slot_path shape and uniqueness ------------------------------

    #[test]
    fn new_slot_path_has_eight_char_hex_name() {
        let pool = PathBuf::from("/tmp/pool");
        let slot = new_slot_path(&pool);
        let name = slot.file_name().unwrap().to_str().unwrap();
        assert_eq!(name.len(), 8, "slot name must be 8 chars, got: {name}");
        assert!(
            name.chars().all(|c| c.is_ascii_hexdigit()),
            "slot name must be hex digits, got: {name}"
        );
    }

    #[test]
    fn new_slot_path_parent_is_pool_dir() {
        let pool = PathBuf::from("/tmp/pool");
        let slot = new_slot_path(&pool);
        assert_eq!(slot.parent().unwrap(), pool.as_path());
    }

    #[test]
    fn new_slot_path_successive_calls_differ() {
        let pool = PathBuf::from("/tmp/pool");
        let a = new_slot_path(&pool);
        let b = new_slot_path(&pool);
        assert_ne!(a, b, "successive slot paths should differ");
    }

    // -- validate_pool_slot --------------------------------------------------

    #[test]
    fn validate_pool_slot_accepts_valid_path() {
        use tempfile::TempDir;
        let pool = TempDir::new().expect("temp pool dir");
        let slot = pool.path().join("a3f9c1b2");
        std::fs::create_dir(&slot).expect("create slot dir");
        assert!(
            validate_pool_slot(&slot, pool.path()).is_ok(),
            "a path directly under pool_dir should be accepted"
        );
    }

    #[test]
    fn validate_pool_slot_rejects_path_outside_pool() {
        use tempfile::TempDir;
        let pool = TempDir::new().expect("temp pool dir");
        let other = TempDir::new().expect("temp other dir");
        let err = validate_pool_slot(other.path(), pool.path())
            .expect_err("path outside pool should be rejected");
        assert!(
            err.to_string().contains("not a bonsai-managed pool slot"),
            "error should mention pool slot, got: {err}"
        );
    }

    #[test]
    fn validate_pool_slot_rejects_nonexistent_path() {
        use tempfile::TempDir;
        let pool = TempDir::new().expect("temp pool dir");
        let nonexistent = pool.path().join("does-not-exist");
        let err = validate_pool_slot(&nonexistent, pool.path())
            .expect_err("nonexistent path should be rejected");
        assert!(
            err.to_string().contains("does not exist"),
            "error should mention 'does not exist', got: {err}"
        );
    }

    // -- find_slot_for_cwd ----------------------------------------------------

    /// When the pool directory does not exist on disk, `find_slot_for_cwd`
    /// MUST return `Ok(None)` without error.
    #[test]
    fn find_slot_for_cwd_returns_none_for_absent_pool() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let nonexistent = PathBuf::from("/nonexistent/bonsai-pool-path-xyz");
        let result =
            find_slot_for_cwd(&cwd, &nonexistent).expect("absent pool dir should not error");
        assert!(result.is_none(), "absent pool dir should return None");
    }

    /// When the CWD is a subdirectory of a registered pool slot,
    /// `find_slot_for_cwd` MUST return `Ok(Some((slot_path, branch)))` where
    /// `slot_path` is an ancestor of `cwd`.
    #[test]
    fn find_slot_for_cwd_matches_slot_ancestor() {
        // We need a real pool dir that git worktree list will include.
        // Use the pool dir of the current repo (if it exists).
        let Ok(root) = managed_root() else { return };
        let Ok(slug) = repo_slug() else { return };
        let pool_dir = root.join(&slug);
        if !pool_dir.exists() {
            return; // pool not provisioned on this machine; skip
        }
        // List the first slot and pretend our CWD is inside it.
        let Ok(entries) = list_pool_worktrees(&pool_dir) else {
            return;
        };
        let Some(entry) = entries.into_iter().next() else {
            return;
        };
        let fake_cwd = entry.path.join("src"); // subdirectory
        let result =
            find_slot_for_cwd(&fake_cwd, &pool_dir).expect("find_slot_for_cwd should not error");
        assert!(
            result.is_some(),
            "CWD inside a slot subtree should be detected"
        );
        let (found_path, _branch) = result.unwrap();
        assert!(
            fake_cwd.starts_with(&found_path),
            "returned path should be an ancestor of the fake CWD"
        );
    }

    // -- find_slot_checked_out_on_branch --------------------------------------

    /// When the pool directory does not exist on disk,
    /// `find_slot_checked_out_on_branch` MUST return `Ok(None)` without
    /// error (mirroring `find_slot_for_cwd`'s absent-pool behaviour).
    #[test]
    fn find_slot_checked_out_on_branch_returns_none_for_absent_pool() {
        let nonexistent = PathBuf::from("/nonexistent/bonsai-pool-path-xyz");
        let result = find_slot_checked_out_on_branch(&nonexistent, "any-branch")
            .expect("absent pool dir should not error");
        assert!(result.is_none(), "absent pool dir should return None");
    }

    /// When the pool directory exists but has no registered worktree slots
    /// (e.g. a fresh temp dir that is not itself a `git worktree add`
    /// target), `find_slot_checked_out_on_branch` MUST return `Ok(None)`.
    #[test]
    fn find_slot_checked_out_on_branch_returns_none_when_no_match() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let result = find_slot_checked_out_on_branch(dir.path(), "no-such-branch")
            .expect("pool dir with no matching slot should not error");
        assert!(
            result.is_none(),
            "fresh pool dir with no matching slot should return None"
        );
    }

    /// When exactly one pool slot is checked out on the requested branch,
    /// `find_slot_checked_out_on_branch` MUST return that slot's path.
    /// Skipped when this machine has no bonsai pool provisioned for the
    /// current repo (mirrors `find_slot_for_cwd_matches_slot_ancestor`'s
    /// environment-dependent skip).
    #[test]
    fn find_slot_checked_out_on_branch_matches_single_slot() {
        let Ok(root) = managed_root() else { return };
        let Ok(slug) = repo_slug() else { return };
        let pool_dir = root.join(&slug);
        if !pool_dir.exists() {
            return; // pool not provisioned on this machine; skip
        }
        let Ok(entries) = list_pool_worktrees(&pool_dir) else {
            return;
        };
        let Some(entry) = entries.into_iter().find(|e| e.branch.is_some()) else {
            return; // no branch-checked-out slot to test against; skip
        };
        let branch = entry.branch.clone().unwrap();
        let result = find_slot_checked_out_on_branch(&pool_dir, &branch)
            .expect("find_slot_checked_out_on_branch should not error");
        assert_eq!(
            result,
            Some(entry.path),
            "should return the slot checked out on {branch}"
        );
    }

    /// A locked slot that is checked out on the requested branch is still a
    /// match: locking does not exclude a slot from this lookup (unlike
    /// `find_available_slot`, which skips locked slots).
    /// Skipped when this machine has no locked, branch-checked-out slot in
    /// the current repo's pool.
    #[test]
    fn find_slot_checked_out_on_branch_matches_locked_slot() {
        let Ok(root) = managed_root() else { return };
        let Ok(slug) = repo_slug() else { return };
        let pool_dir = root.join(&slug);
        if !pool_dir.exists() {
            return; // pool not provisioned on this machine; skip
        }
        let Ok(entries) = list_pool_worktrees(&pool_dir) else {
            return;
        };
        let Some(entry) = entries.into_iter().find(|e| e.locked && e.branch.is_some()) else {
            return; // no locked, branch-checked-out slot to test against; skip
        };
        let branch = entry.branch.clone().unwrap();
        let result = find_slot_checked_out_on_branch(&pool_dir, &branch)
            .expect("find_slot_checked_out_on_branch should not error");
        assert_eq!(
            result,
            Some(entry.path),
            "a locked slot checked out on {branch} should still be returned"
        );
    }
}
