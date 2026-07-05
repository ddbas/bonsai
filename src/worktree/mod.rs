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
    /// Slot is locked or has uncommitted changes.
    InUse,
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

/// Parse `git worktree list --porcelain` and return entries whose path falls
/// under `pool_dir`.
pub fn list_pool_worktrees(pool_dir: &Path) -> Result<Vec<WorktreeEntry>> {
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
    let mut cur_path: Option<PathBuf> = None;
    let mut cur_locked = false;
    let mut cur_branch: Option<String> = None;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            // Flush the previous entry when we hit the next `worktree` line.
            if let Some(path) = cur_path.take() {
                let canonical = path.canonicalize().unwrap_or(path);
                if canonical.starts_with(&pool_canonical) {
                    entries.push(WorktreeEntry {
                        path: canonical,
                        locked: cur_locked,
                        branch: cur_branch.take(),
                    });
                }
            }
            cur_path = Some(PathBuf::from(rest.trim()));
            cur_locked = false;
            cur_branch = None;
        } else if line.starts_with("locked") {
            cur_locked = true;
        } else if let Some(refs) = line.strip_prefix("branch ") {
            // `branch refs/heads/main` → `Some("main")`
            let short = refs
                .trim()
                .strip_prefix("refs/heads/")
                .unwrap_or(refs.trim());
            cur_branch = Some(short.to_string());
        }
        // `detached` line → cur_branch stays None
    }

    // Flush the last entry.
    if let Some(path) = cur_path.take() {
        let canonical = path.canonicalize().unwrap_or(path);
        if canonical.starts_with(&pool_canonical) {
            entries.push(WorktreeEntry {
                path: canonical,
                locked: cur_locked,
                branch: cur_branch.take(),
            });
        }
    }

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
/// untracked files, no open process handles); otherwise it is
/// [`WorktreeStatus::InUse`].
///
/// The returned tuple is `(path, status, stats, branch)` where `branch` is
/// the short checked-out branch name (`None` for detached HEAD).
pub fn list_worktrees_status(pool_dir: &Path) -> Result<Vec<WorktreeListEntry>> {
    let entries = list_pool_worktrees(pool_dir)?;
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries {
        let branch = entry.branch.clone();
        let (status, stats) = if entry.locked || !entry.path.exists() {
            (WorktreeStatus::InUse, WorktreeStats::zero())
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
        result.push((entry.path, status, stats, branch));
    }
    Ok(result)
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

/// Internal helper: run `lsof_bin +D <path>` and return the number of distinct
/// PIDs with open file descriptors under `path`.
///
/// Separated from `count_open_processes` so tests can pass a non-existent
/// binary name without mutating the global `PATH` environment variable.
fn run_lsof_count(lsof_bin: &str, path: &Path) -> Result<usize> {
    let output = Command::new(lsof_bin)
        .args(["+D", &path.to_string_lossy()])
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
        if stderr.trim().is_empty() {
            return Ok(0);
        } else {
            bail!("lsof error for {}: {}", path.display(), stderr.trim());
        }
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_lsof_pids(&text))
}

/// Return the number of distinct PIDs that have open file descriptors anywhere
/// under `path` (including `path` itself).
///
/// Uses `lsof +D <path>` to recursively query open file handles.  The PID
/// (second whitespace-delimited column) of each non-header output line is
/// collected into a `HashSet`; the size of that set is returned.
///
/// - Non-empty stdout → parses and returns the distinct-PID count.
/// - Empty stdout + empty stderr → no files are open → returns `Ok(0)`.
/// - Spawn error (`lsof` not on `PATH`) → returns `Err` with an actionable
///   message naming `lsof` as the missing dependency.
/// - Non-empty stderr → `lsof` itself encountered an error → returns `Err`.
pub fn count_open_processes(path: &Path) -> Result<usize> {
    run_lsof_count("lsof", path)
}

/// Internal helper: run `lsof_bin +D <path>` and return whether any process
/// has open file descriptors under `path`.
///
/// Separated from `has_open_files` so tests can pass a non-existent binary
/// name without mutating the global `PATH` environment variable.
fn run_lsof(lsof_bin: &str, path: &Path) -> Result<bool> {
    let output = Command::new(lsof_bin)
        .args(["+D", &path.to_string_lossy()])
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
    // 1. Non-empty stdout  → lsof found at least one open file descriptor
    //    → `Ok(true)`.
    // 2. Empty stdout + empty stderr  → no open file descriptors
    //    → `Ok(false)`.
    // 3. Non-empty stderr  → lsof encountered a real error (e.g. the path
    //    does not exist, or a permission error)  → `Err(...)`.
    if !output.stdout.is_empty() {
        return Ok(true);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.trim().is_empty() {
        Ok(false)
    } else {
        bail!("lsof error for {}: {}", path.display(), stderr.trim())
    }
}

/// Detect whether any process currently has an open file descriptor under
/// `path` (including `path` itself).
///
/// Uses `lsof +D <path>` to recursively query open file handles.  The
/// result is determined by stdout/stderr content (not exit code, which
/// `lsof` sets unreliably across platforms):
///
/// - Non-empty stdout → at least one process has the path open →
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

/// Return the first available (clean, unlocked, on-disk) slot in the pool,
/// or `None` if every slot is unavailable.
///
/// Runs `git worktree prune` first to remove stale entries.
pub fn find_available_slot(pool_dir: &Path) -> Result<Option<PathBuf>> {
    prune_worktrees()?;
    for entry in list_pool_worktrees(pool_dir)? {
        if entry.locked || !entry.path.exists() {
            continue;
        }
        if !is_clean(&entry.path)? {
            continue;
        }
        if count_open_processes(&entry.path)? > 0 {
            continue;
        }
        return Ok(Some(entry.path));
    }
    Ok(None)
}

// -- Provision (section 4) ----------------------------------------------------

/// Reset an existing slot to detached HEAD at `head_sha`.
///
/// Runs `git -C <slot> checkout --detach <head_sha>`.
pub fn reset_slot(slot_path: &Path, head_sha: &str) -> Result<()> {
    let output = git_cmd()
        .args([
            "-C",
            &slot_path.to_string_lossy(),
            "checkout",
            "--detach",
            head_sha,
        ])
        .output()
        .context("failed to spawn `git checkout --detach`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git checkout --detach` failed: {}", stderr.trim());
    }

    Ok(())
}

/// Create a new worktree slot at `slot_path` in detached HEAD state.
///
/// Runs `git worktree add --detach <slot_path> <head_sha>`.
pub fn create_slot(slot_path: &Path, head_sha: &str) -> Result<()> {
    let output = git_cmd()
        .args([
            "worktree",
            "add",
            "--detach",
            &slot_path.to_string_lossy(),
            head_sha,
        ])
        .output()
        .context("failed to spawn `git worktree add --detach`")?;

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
/// 1. Resolve `HEAD` SHA.
/// 2. Derive pool directory (`managed_root()/<repo-slug>/`).
/// 3. Create pool directory if it does not yet exist.
/// 4. Scan for an available slot; if none, generate a new UUID slot.
/// 5. Reset (or add) the slot to `HEAD` in detached state.
/// 6. Return the canonicalised slot path.
pub fn get_worktree() -> Result<PathBuf> {
    let head_sha = resolve_head()?;
    let slug = repo_slug()?;
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

    let slot_path = match find_available_slot(&pool_dir)? {
        Some(existing) => {
            reset_slot(&existing, &head_sha)?;
            existing
        }
        None => {
            let new_slot = new_slot_path(&pool_dir);
            create_slot(&new_slot, &head_sha)?;
            new_slot
        }
    };

    slot_path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize slot path {}", slot_path.display()))
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
    fn status_locked_is_in_use() {
        let (status, stats) = synthetic_status(true, true, 0, 0, 0);
        assert_eq!(status, WorktreeStatus::InUse);
        assert_eq!(stats.process_count, 0);
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
        if locked || !exists {
            (WorktreeStatus::InUse, WorktreeStats::zero())
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

    // -- 7.2: slug normalisation ----------------------------------------------

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
}
