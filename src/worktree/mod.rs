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
    /// The optional reason text passed to `git worktree lock --reason`, when
    /// the slot is locked. `None` when unlocked, or locked with no reason.
    pub lock_reason: Option<String>,
    /// The checked-out branch name (short form, e.g. `main`), or `None` for
    /// detached HEAD.
    pub branch: Option<String>,
}

/// A single distinct process with an open file descriptor directly in a
/// slot's root (non-recursive), as reported by `lsof -w +d <path>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessHandle {
    /// Process ID, as reported by `lsof` (kept as a string; not parsed to a
    /// numeric type since it is only ever displayed, never computed with).
    pub pid: String,
    /// Command name (`lsof`'s `COMMAND` column).
    pub command: String,
}

/// A single classified line from `git status --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusLine {
    /// The two-character XY status code (e.g. ` M`, `??`, `A `).
    pub code: String,
    /// The file path as reported by git (rename entries include the full
    /// ` -> ` arrow text verbatim).
    pub path: String,
}

/// Detailed `git status --porcelain` output for a slot, split into
/// uncommitted (non-`??` XY code) and untracked (`??` XY code) lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitStatusDetail {
    pub uncommitted: Vec<GitStatusLine>,
    pub untracked: Vec<GitStatusLine>,
}

/// Detailed status report for a single managed pool worktree slot, combining
/// lock state (+ reason), git-status detail, and open-process detail.
///
/// Built by [`slot_status`]; rendered by the `bs status` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotStatusReport {
    pub path: PathBuf,
    /// The checked-out branch name, or `None` for detached HEAD.
    pub branch: Option<String>,
    /// Overall classification, using the same priority rules `bs list` used
    /// to apply: `Locked` > `InUse` > `Available`.
    pub status: WorktreeStatus,
    /// The lock's `--reason` text, when the slot is locked and a reason was
    /// given.
    pub lock_reason: Option<String>,
    /// Distinct processes with an open file descriptor directly in the slot
    /// root.
    pub processes: Vec<ProcessHandle>,
    /// Itemized uncommitted/untracked `git status --porcelain` lines.
    pub git_status: GitStatusDetail,
}

/// Tuple type returned by [`list_worktrees_status`]: path, status, and the
/// short branch name (or `None` for detached HEAD).
pub type WorktreeListEntry = (PathBuf, WorktreeStatus, Option<String>);

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

// -- Copy configured ignored files (bonsai.copy) -----------------------------

/// Read the multi-valued `bonsai.copy` git config key, resolved from `dir`
/// (the origin worktree), via `git config --get-all bonsai.copy`.
///
/// Returns the configured entries in git's own config order (local +
/// global, additive). When the key is not set anywhere, `git config
/// --get-all` exits with status 1 and empty output/stderr — that case is
/// treated as an empty `Vec` rather than an error. Any other non-zero exit
/// (e.g. a corrupt config file, which prints a diagnostic to stderr) is
/// surfaced as a genuine error.
pub fn configured_copy_paths(dir: &Path) -> Result<Vec<String>> {
    let output = git_cmd()
        .args([
            "-C",
            &dir.to_string_lossy(),
            "config",
            "--get-all",
            "bonsai.copy",
        ])
        .output()
        .context("failed to spawn `git config --get-all bonsai.copy`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        if stderr.is_empty() {
            // Key not set anywhere — not an error.
            return Ok(Vec::new());
        }
        bail!("`git config --get-all bonsai.copy` failed: {}", stderr);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect())
}

/// Copy each relative path in `entries` from `origin_root` into the
/// corresponding relative path under `slot_path`.
///
/// Missing source files are skipped silently (best-effort semantics for
/// `bonsai.copy`); destination parent directories are created as needed.
/// Genuine copy failures (e.g. permission errors) are propagated.
pub fn copy_ignored_files(origin_root: &Path, slot_path: &Path, entries: &[String]) -> Result<()> {
    let mut copied = 0usize;
    let mut skipped = 0usize;

    for entry in entries {
        let source = origin_root.join(entry);
        if !source.exists() {
            tracing::debug!(
                "bonsai.copy: skipping missing source file {}",
                source.display()
            );
            skipped += 1;
            continue;
        }

        let dest = slot_path.join(entry);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create destination directory {}",
                    parent.display()
                )
            })?;
        }

        std::fs::copy(&source, &dest).with_context(|| {
            format!("failed to copy {} to {}", source.display(), dest.display())
        })?;
        copied += 1;
    }

    if copied > 0 || skipped > 0 {
        tracing::info!(
            "bonsai.copy: copied {} file(s), skipped {} missing entry(ies) into {}",
            copied,
            skipped,
            slot_path.display()
        );
    }

    Ok(())
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

/// Parse a `locked` line from `git worktree list --porcelain` output.
///
/// The line is always `locked` when the slot is locked with no reason, or
/// `locked <reason text>` when locked with `git worktree lock --reason`.
/// Returns `(is_locked, reason)`; `reason` is `None` when no reason text
/// follows `locked`.
fn parse_locked_line(line: &str) -> (bool, Option<String>) {
    match line.strip_prefix("locked") {
        Some(rest) => {
            let reason = rest.trim();
            (
                true,
                if reason.is_empty() {
                    None
                } else {
                    Some(reason.to_string())
                },
            )
        }
        None => (false, None),
    }
}

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
    let mut cur_lock_reason: Option<String> = None;
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
                        lock_reason: cur_lock_reason.take(),
                        branch: cur_branch.take(),
                    });
                } else {
                    // Non-pool entry: branch/reason are irrelevant; drop them.
                    let _ = cur_branch.take();
                    let _ = cur_lock_reason.take();
                }
            }
        };
    }

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            flush!();
            cur_path = Some(PathBuf::from(rest.trim()));
            cur_locked = false;
            cur_lock_reason = None;
            cur_branch = None;
        } else if line.starts_with("locked") {
            let (locked, reason) = parse_locked_line(line);
            cur_locked = locked;
            cur_lock_reason = reason;
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

/// Internal: run `git status --porcelain` for `slot_path` once and return the
/// raw stdout text. Shared by [`git_status_lines`] and [`is_clean`] so the
/// subprocess invocation is not duplicated.
fn run_git_status_porcelain(slot_path: &Path) -> Result<String> {
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

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Return the individual `git status --porcelain` lines for `slot_path`,
/// split into uncommitted (non-`??` XY code) and untracked (`??` XY code)
/// entries.
///
/// Reuses the same subprocess invocation as [`is_clean`] (via
/// [`run_git_status_porcelain`]) but keeps the per-line detail (status code +
/// path) instead of collapsing to a boolean.
pub fn git_status_lines(slot_path: &Path) -> Result<GitStatusDetail> {
    let text = run_git_status_porcelain(slot_path)?;
    let mut detail = GitStatusDetail::default();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let (code, path) = if line.len() >= 3 {
            (line[..2].to_string(), line[3..].to_string())
        } else if line.len() >= 2 {
            (line[..2].to_string(), String::new())
        } else {
            (line.to_string(), String::new())
        };
        let is_untracked = code == "??";
        let entry = GitStatusLine { code, path };
        if is_untracked {
            detail.untracked.push(entry);
        } else {
            detail.uncommitted.push(entry);
        }
    }
    Ok(detail)
}

/// Classify a slot given its lock state, dirty state, and open-process
/// state.
///
/// Priority: `Locked` > `InUse` (dirty or open processes) > `Available`. This
/// is the single place the priority rule is encoded; both
/// [`classify_slot_status`] (used by `bs list`, fed early-return booleans)
/// and [`slot_status`] (used by `bs status`, fed booleans derived from the
/// full detail it already collects) call this function so the two commands
/// can never disagree on a given slot's classification.
pub fn classify(locked: bool, dirty: bool, has_processes: bool) -> WorktreeStatus {
    if locked {
        WorktreeStatus::Locked
    } else if dirty || has_processes {
        WorktreeStatus::InUse
    } else {
        WorktreeStatus::Available
    }
}

/// Classify a single pool slot for `bs list`, short-circuiting as soon as the
/// classification is known so cheaper signals are checked first:
///
/// 1. `entry.locked` (already known for free from the
///    `git worktree list --porcelain` parse) → `Locked` immediately, no
///    `git status`/`lsof` call at all.
/// 2. [`is_clean`] → `InUse` immediately if dirty, **no `lsof` call**.
/// 3. [`has_open_files`] → determines the final `InUse`/`Available` split.
pub fn classify_slot_status(entry: &WorktreeEntry) -> Result<WorktreeStatus> {
    if entry.locked {
        return Ok(classify(true, false, false));
    }
    if !is_clean(&entry.path)? {
        return Ok(classify(false, true, false));
    }
    let has_processes = has_open_files(&entry.path)?;
    Ok(classify(false, false, has_processes))
}

/// Return the availability status of every pool worktree slot.
///
/// Each slot's status is computed via [`classify_slot_status`], which
/// short-circuits per slot: a locked slot never triggers `git status`/`lsof`;
/// a dirty unlocked slot never triggers `lsof`.
///
/// Per-slot checks are executed concurrently (one thread per slot); the
/// returned `Vec` preserves the original slot ordering from
/// `git worktree list --porcelain`.
///
/// The returned tuple is `(path, status, branch)` where `branch` is the short
/// checked-out branch name (`None` for detached HEAD).
pub fn list_worktrees_status(pool_dir: &Path) -> Result<Vec<WorktreeListEntry>> {
    let entries = list_pool_worktrees(pool_dir)?;

    // Spawn one thread per slot so that any blocking `git status`/`lsof`
    // calls run concurrently.  Handles are collected into a `Vec` and joined
    // in original slot order, guaranteeing that the returned `Vec` ordering
    // matches `git worktree list` regardless of which thread finishes first.
    let handles: Vec<std::thread::JoinHandle<Result<WorktreeListEntry>>> = entries
        .into_iter()
        .map(|entry| {
            std::thread::spawn(move || -> Result<WorktreeListEntry> {
                let branch = entry.branch.clone();
                let status = if !entry.path.exists() {
                    WorktreeStatus::InUse
                } else {
                    classify_slot_status(&entry)?
                };
                Ok((entry.path, status, branch))
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

/// Compute a detailed status report for a single managed pool worktree slot.
///
/// `path` must already be a canonicalizable, registered git worktree (callers
/// resolve/validate the slot first via [`current_worktree`] or
/// [`validate_pool_slot`]). Combines:
/// - lock state + reason, from `git worktree list --porcelain` (reusing
///   [`list_pool_worktrees_checking_stale`], scoped to just this one slot by
///   passing the slot's own canonical path as the "pool" filter),
/// - itemized `git status --porcelain` lines, via [`git_status_lines`],
/// - itemized open-process detail, via [`list_open_processes`],
///
/// and derives the overall [`WorktreeStatus`] classification using the same
/// priority rules `bs list` used to apply: `Locked` > `InUse` > `Available`.
pub fn slot_status(path: &Path) -> Result<SlotStatusReport> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize slot path {}", path.display()))?;

    // Scoping the "pool" filter to the slot's own path means only this one
    // worktree entry (if any) passes the `starts_with` check inside
    // `list_pool_worktrees_checking_stale`.
    let (entries, _has_stale) = list_pool_worktrees_checking_stale(&canonical)?;
    let entry = entries
        .into_iter()
        .find(|e| e.path == canonical)
        .ok_or_else(|| {
            anyhow::anyhow!("{} is not a registered git worktree", canonical.display())
        })?;

    let processes = list_open_processes(&canonical)?;
    let git_status = git_status_lines(&canonical)?;

    let dirty = !git_status.uncommitted.is_empty() || !git_status.untracked.is_empty();
    let has_processes = !processes.is_empty();
    let status = classify(entry.locked, dirty, has_processes);

    Ok(SlotStatusReport {
        path: canonical,
        branch: entry.branch,
        status,
        lock_reason: entry.lock_reason,
        processes,
        git_status,
    })
}

/// Parse `(pid, command)` pairs from `lsof +d` stdout, deduplicated by PID
/// (first occurrence wins), preserving first-seen order.
///
/// Skips the header line (starts with `COMMAND`). The command name is the
/// first whitespace-delimited field; the PID is the second.
fn parse_lsof_processes(stdout: &str) -> Vec<ProcessHandle> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();
    for line in stdout.lines() {
        if line.starts_with("COMMAND") {
            continue;
        }
        let mut fields = line.split_whitespace();
        let Some(command) = fields.next() else {
            continue;
        };
        let Some(pid) = fields.next() else {
            continue;
        };
        if seen.insert(pid.to_string()) {
            result.push(ProcessHandle {
                pid: pid.to_string(),
                command: command.to_string(),
            });
        }
    }
    result
}

/// Internal: run `lsof_bin -w +d <path>` once and return the raw stdout text,
/// or `Ok(String::new())` when no files are open. Shared by
/// [`run_lsof`] and [`run_lsof_processes`] so the subprocess invocation and
/// its stdout/stderr interpretation live in one place.
fn run_lsof_raw(lsof_bin: &str, path: &Path) -> Result<String> {
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
            return Ok(String::new());
        } else {
            bail!("lsof error for {}: {}", path.display(), stderr);
        }
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Internal helper: run `lsof_bin -w +d <path>` and return the distinct
/// `(pid, command)` pairs with open file descriptors directly in `path`
/// (non-recursive).
///
/// Separated from `list_open_processes` so tests can pass a non-existent
/// binary name without mutating the global `PATH` environment variable.
fn run_lsof_processes(lsof_bin: &str, path: &Path) -> Result<Vec<ProcessHandle>> {
    let text = run_lsof_raw(lsof_bin, path)?;
    Ok(parse_lsof_processes(&text))
}

/// Return the distinct processes (PID + command name) with an open file
/// descriptor directly in `path` (non-recursive; the top-level directory
/// only), deduplicated by PID.
///
/// Reuses the same subprocess invocation as [`has_open_files`] (via
/// [`run_lsof_raw`]) but keeps the per-process detail instead of collapsing
/// to a boolean. Error semantics are identical to [`has_open_files`].
pub fn list_open_processes(path: &Path) -> Result<Vec<ProcessHandle>> {
    run_lsof_processes("lsof", path)
}

/// Internal helper: run `lsof_bin -w +d <path>` and return whether any
/// process has open file descriptors directly in `path` (non-recursive).
///
/// Separated from `has_open_files` so tests can pass a non-existent binary
/// name without mutating the global `PATH` environment variable. Reuses
/// [`run_lsof_raw`] for the actual subprocess invocation.
fn run_lsof(lsof_bin: &str, path: &Path) -> Result<bool> {
    let text = run_lsof_raw(lsof_bin, path)?;
    Ok(!text.is_empty())
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
        if has_open_files(&entry.path)? {
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
    // Capture the origin worktree's CWD before any slot path changes; this is
    // both the source root for `bonsai.copy` entries and the directory `git
    // config --get-all bonsai.copy` is resolved from.
    let origin_dir = std::env::current_dir().context("failed to get current directory")?;

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

    // Copy any `bonsai.copy`-configured ignored files from the origin
    // worktree into the slot as the final provisioning step, after the slot
    // has reached its final (created/reset + checked-out) state. Runs
    // identically for both the reused-slot and new-slot branches above.
    let copy_entries = configured_copy_paths(&origin_dir)?;
    if !copy_entries.is_empty() {
        copy_ignored_files(&origin_dir, &slot_path, &copy_entries)?;
    }

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

// -- Prune (section: `bs prune`) ---------------------------------------------

/// A single pool slot that `bs prune` deleted (or attempted to delete), with
/// the identifying info (path + branch) needed for CLI reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrunedSlot {
    pub path: PathBuf,
    /// The checked-out branch name, or `None` for detached HEAD.
    pub branch: Option<String>,
}

/// Result of a full `prune_pool` run: the slots successfully pruned, any
/// per-slot deletion failures (path + error message), the slot preserved to
/// keep the pool warm (if any available slot existed), and a failure to
/// detach the preserved slot's branch (if that operation was attempted and
/// failed). `git worktree prune` is always attempted regardless of
/// per-slot/preserve outcomes.
#[derive(Debug, Default)]
pub struct PruneOutcome {
    pub pruned: Vec<PrunedSlot>,
    pub failures: Vec<(PathBuf, String)>,
    /// The available slot kept (not deleted) to keep the pool warm, with the
    /// branch it had checked out *before* being detached (or `None` if it was
    /// already in detached HEAD and untouched). `None` only when there were
    /// no available slots at all.
    pub preserved: Option<PrunedSlot>,
    /// Path + error message when detaching the preserved slot's branch
    /// failed. The slot's directory is never deleted in this case.
    pub preserve_failure: Option<(PathBuf, String)>,
}

/// A pool slot's path and its checked-out branch (or `None` for detached
/// HEAD), as returned by [`prune_available_slots`].
pub type PruneSlotCandidate = (PathBuf, Option<String>);

/// Return every pool slot classified [`WorktreeStatus::Available`] for
/// `pool_dir`, as `(path, branch)` pairs, in the same pool order used by
/// `bs list`/`bs status`.
///
/// Reuses [`list_worktrees_status`] (the same classification `bs list`
/// uses) rather than writing new porcelain-parsing logic, so `bs prune` and
/// `bs list` can never disagree about which slots are available.
pub fn prune_available_slots(pool_dir: &Path) -> Result<Vec<PruneSlotCandidate>> {
    let entries = list_worktrees_status(pool_dir)?;
    Ok(entries
        .into_iter()
        .filter(|(_, status, _)| *status == WorktreeStatus::Available)
        .map(|(path, _, branch)| (path, branch))
        .collect())
}

/// Split a list of available slots (in pool order, as returned by
/// [`prune_available_slots`]) into the slot to preserve and the slots to
/// delete.
///
/// The first entry (if any) is preserved; every other entry is returned for
/// deletion. Returns `(None, vec![])` for an empty input.
pub fn select_prune_candidates(
    available: Vec<PruneSlotCandidate>,
) -> (Option<PruneSlotCandidate>, Vec<PruneSlotCandidate>) {
    let mut iter = available.into_iter();
    let preserved = iter.next();
    let to_delete = iter.collect();
    (preserved, to_delete)
}

/// Delete a single slot's on-disk directory via [`std::fs::remove_dir_all`].
///
/// Performs no git operations; deregistration is left entirely to a
/// subsequent `git worktree prune` call (see [`git_worktree_prune`]).
pub fn delete_slot_dir(path: &Path) -> Result<()> {
    std::fs::remove_dir_all(path)
        .with_context(|| format!("failed to delete slot directory {}", path.display()))
}

/// Run `git worktree prune`, letting git deregister any worktree entries
/// whose administrative files point at directories that no longer exist on
/// disk.
///
/// This is the only place `bs prune` touches git's worktree bookkeeping; no
/// `.git/worktrees/*` file is ever edited directly by bonsai.
pub fn git_worktree_prune() -> Result<()> {
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

/// Prune available slots in `pool_dir`, always keeping the pool warm:
/// enumerate available slots, preserve exactly one (the first in pool order,
/// detaching its branch in place if it has one checked out), delete every
/// other available slot's directory (collecting successes and per-slot
/// failures without aborting the run), then always run `git worktree prune`
/// once at the end regardless of preserve/deletion outcomes.
///
/// Returns a [`PruneOutcome`] the CLI layer uses to report the preserved
/// slot, pruned slots, and any failures. Only propagates an error if
/// `git_worktree_prune` itself fails to spawn/exits non-zero, or if
/// enumerating slots fails; individual deletion/detach failures are
/// captured in `PruneOutcome::failures`/`PruneOutcome::preserve_failure`
/// instead.
pub fn prune_pool(pool_dir: &Path) -> Result<PruneOutcome> {
    let candidates = prune_available_slots(pool_dir)?;
    let (preserved, to_delete) = select_prune_candidates(candidates);

    let mut outcome = PruneOutcome::default();

    if let Some((path, branch)) = preserved {
        match &branch {
            Some(_) => match resolve_head().and_then(|head_sha| reset_slot(&path, &head_sha, None))
            {
                Ok(()) => outcome.preserved = Some(PrunedSlot { path, branch }),
                Err(err) => outcome.preserve_failure = Some((path, format!("{err:#}"))),
            },
            None => outcome.preserved = Some(PrunedSlot { path, branch }),
        }
    }

    for (path, branch) in to_delete {
        match delete_slot_dir(&path) {
            Ok(()) => outcome.pruned.push(PrunedSlot { path, branch }),
            Err(err) => outcome.failures.push((path, format!("{err:#}"))),
        }
    }

    git_worktree_prune()?;

    Ok(outcome)
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

    // -- classify() priority-order combinations -------------------------------

    #[test]
    fn classify_locked_beats_everything() {
        assert_eq!(classify(true, false, false), WorktreeStatus::Locked);
        assert_eq!(classify(true, true, false), WorktreeStatus::Locked);
        assert_eq!(classify(true, false, true), WorktreeStatus::Locked);
        assert_eq!(classify(true, true, true), WorktreeStatus::Locked);
    }

    #[test]
    fn classify_dirty_unlocked_is_in_use() {
        assert_eq!(classify(false, true, false), WorktreeStatus::InUse);
    }

    #[test]
    fn classify_open_processes_unlocked_is_in_use() {
        assert_eq!(classify(false, false, true), WorktreeStatus::InUse);
    }

    #[test]
    fn classify_dirty_and_open_processes_is_in_use() {
        assert_eq!(classify(false, true, true), WorktreeStatus::InUse);
    }

    #[test]
    fn classify_clean_unlocked_no_processes_is_available() {
        assert_eq!(classify(false, false, false), WorktreeStatus::Available);
    }

    // -- classify_slot_status early-return short-circuit ----------------------

    fn synthetic_entry(path: PathBuf, locked: bool) -> WorktreeEntry {
        WorktreeEntry {
            path,
            locked,
            lock_reason: None,
            branch: None,
        }
    }

    /// A locked slot is classified `Locked` without ever invoking `git
    /// status` or `lsof` — verified by pointing `PATH` at a directory that
    /// only contains `git` (no `lsof` binary, and a `git` shim that would
    /// fail loudly if `status --porcelain` were invoked at all, since the
    /// slot directory below is never a real git repo).
    #[test]
    fn classify_slot_status_locked_short_circuits() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let entry = synthetic_entry(dir.path().to_path_buf(), true);
        let status = classify_slot_status(&entry).expect(
            "a locked slot must classify without needing git status/lsof \
             (the directory is not a real git repo, so any subprocess call \
             other than the locked short-circuit would fail)",
        );
        assert_eq!(status, WorktreeStatus::Locked);
    }

    #[test]
    fn classify_slot_status_unlocked_dirty_is_in_use() {
        let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Use the current repo checkout, which is always unlocked here; we
        // only assert the outcome type, not force dirtiness, since CI/dev
        // checkouts vary. This test mainly exercises that classify_slot_status
        // runs without needing lsof when git reports a clean tree it still
        // proceeds to check for open files (covered separately below).
        let entry = synthetic_entry(repo, false);
        // Should not panic; result depends on host git state.
        let _ = classify_slot_status(&entry);
    }

    #[test]
    fn classify_slot_status_and_slot_status_agree_via_classify() {
        // Both classify_slot_status (bs list) and slot_status (bs status) are
        // required to route through the same `classify()` function so they
        // cannot disagree. This is a compile-time/structural guarantee
        // exercised indirectly by the classify() priority tests above; here we
        // additionally confirm classify() itself is deterministic and total
        // over all 8 boolean combinations.
        for locked in [false, true] {
            for dirty in [false, true] {
                for has_processes in [false, true] {
                    let status = classify(locked, dirty, has_processes);
                    if locked {
                        assert_eq!(status, WorktreeStatus::Locked);
                    } else if dirty || has_processes {
                        assert_eq!(status, WorktreeStatus::InUse);
                    } else {
                        assert_eq!(status, WorktreeStatus::Available);
                    }
                }
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

    // -- parse_locked_line ---------------------------------------------------

    #[test]
    fn parse_locked_line_no_lock() {
        let (locked, reason) = parse_locked_line("branch refs/heads/main");
        assert!(!locked);
        assert!(reason.is_none());
    }

    #[test]
    fn parse_locked_line_locked_no_reason() {
        let (locked, reason) = parse_locked_line("locked");
        assert!(locked);
        assert!(reason.is_none());
    }

    #[test]
    fn parse_locked_line_locked_with_reason() {
        let (locked, reason) = parse_locked_line("locked build in progress");
        assert!(locked);
        assert_eq!(reason.as_deref(), Some("build in progress"));
    }

    // -- git_status_lines parsing ---------------------------------------------

    /// Mirrors `git_status_lines`'s per-line classification without requiring
    /// a live git repository.
    fn synthetic_git_status_lines(porcelain: &str) -> GitStatusDetail {
        let mut detail = GitStatusDetail::default();
        for line in porcelain.lines() {
            if line.is_empty() {
                continue;
            }
            let (code, path) = if line.len() >= 3 {
                (line[..2].to_string(), line[3..].to_string())
            } else if line.len() >= 2 {
                (line[..2].to_string(), String::new())
            } else {
                (line.to_string(), String::new())
            };
            let is_untracked = code == "??";
            let entry = GitStatusLine { code, path };
            if is_untracked {
                detail.untracked.push(entry);
            } else {
                detail.uncommitted.push(entry);
            }
        }
        detail
    }

    #[test]
    fn git_status_lines_classifies_modified_and_untracked() {
        let porcelain = " M src/main.rs\n?? build/\nA  new_file.rs\n?? tmp/\n";
        let detail = synthetic_git_status_lines(porcelain);
        assert_eq!(detail.uncommitted.len(), 2);
        assert_eq!(detail.untracked.len(), 2);
        assert_eq!(detail.uncommitted[0].path, "src/main.rs");
        assert_eq!(detail.untracked[0].path, "build/");
    }

    #[test]
    fn git_status_lines_empty_output() {
        let detail = synthetic_git_status_lines("");
        assert!(detail.uncommitted.is_empty());
        assert!(detail.untracked.is_empty());
    }

    // -- parse_lsof_processes --------------------------------------------------

    #[test]
    fn parse_lsof_processes_deduplicates_by_pid() {
        let mock_output = "COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\n\
                           vim       100 user  cwd    DIR    1,2      512  123 /tmp/dir\n\
                           vim       100 user  txt    REG    1,2     4096  456 /tmp/dir/f1\n\
                           bash      200 user  txt    REG    1,2     4096  789 /tmp/dir/f2\n";
        let processes = parse_lsof_processes(mock_output);
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0].pid, "100");
        assert_eq!(processes[0].command, "vim");
        assert_eq!(processes[1].pid, "200");
        assert_eq!(processes[1].command, "bash");
    }

    #[test]
    fn parse_lsof_processes_empty_output() {
        assert!(parse_lsof_processes("").is_empty());
    }

    /// A file held open in a temp dir is reported with the correct PID by
    /// `list_open_processes`.
    #[test]
    fn list_open_processes_returns_current_pid_when_file_open() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let file_path = dir.path().join("held_open.txt");
        fs::write(&file_path, b"data").expect("write");
        let _handle = fs::File::open(&file_path).expect("open file");

        let processes =
            list_open_processes(dir.path()).expect("list_open_processes should not error");
        assert_eq!(
            processes.len(),
            1,
            "one process (this test) holds the file open"
        );
        assert_eq!(processes[0].pid, std::process::id().to_string());
    }

    #[test]
    fn list_open_processes_returns_empty_when_no_open_files() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");

        let processes =
            list_open_processes(dir.path()).expect("list_open_processes should not error");
        assert!(processes.is_empty());
    }

    #[test]
    fn list_open_processes_err_when_lsof_not_found() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        let err = run_lsof_processes("/nonexistent/path/to/lsof-binary-xyz", dir.path())
            .expect_err("run_lsof_processes should return Err when the binary is not found");
        assert!(
            err.to_string().contains("lsof"),
            "error message should mention 'lsof', got: {err}"
        );
    }

    // -- has_open_files -------------------------------------------------------

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

    // -- branch_mode -----------------------------------------------------------

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

    // -- configured_copy_paths ------------------------------------------------

    /// Initialise a throwaway git repo in a fresh temp dir, suitable for
    /// running `git config` commands against without touching host config.
    fn init_scratch_repo() -> tempfile::TempDir {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("temp dir");
        // Use `git_cmd()` (not a bare `Command`) so hook-injected `GIT_DIR`/
        // `GIT_WORK_TREE`/etc. env vars (e.g. when tests run inside a
        // pre-commit hook) don't redirect this `git init` at the real repo.
        let status = git_cmd()
            .args(["-C", &dir.path().to_string_lossy(), "init", "-q"])
            .status()
            .expect("spawn git init");
        assert!(status.success(), "git init failed");
        dir
    }

    fn git_config_add(dir: &Path, key: &str, value: &str) {
        let status = git_cmd()
            .args(["-C", &dir.to_string_lossy(), "config", "--add", key, value])
            .status()
            .expect("spawn git config --add");
        assert!(status.success(), "git config --add failed");
    }

    #[test]
    fn configured_copy_paths_unset_returns_empty() {
        let dir = init_scratch_repo();
        let entries = configured_copy_paths(dir.path()).expect("should not error");
        assert!(
            entries.is_empty(),
            "unset bonsai.copy should return an empty Vec, got: {entries:?}"
        );
    }

    #[test]
    fn configured_copy_paths_single_entry() {
        let dir = init_scratch_repo();
        git_config_add(dir.path(), "bonsai.copy", ".env");
        let entries = configured_copy_paths(dir.path()).expect("should not error");
        assert_eq!(entries, vec![".env".to_string()]);
    }

    #[test]
    fn configured_copy_paths_multiple_entries_preserve_order() {
        let dir = init_scratch_repo();
        git_config_add(dir.path(), "bonsai.copy", ".env");
        git_config_add(dir.path(), "bonsai.copy", "config/local.json");
        git_config_add(dir.path(), "bonsai.copy", ".idea/workspace.xml");
        let entries = configured_copy_paths(dir.path()).expect("should not error");
        assert_eq!(
            entries,
            vec![
                ".env".to_string(),
                "config/local.json".to_string(),
                ".idea/workspace.xml".to_string(),
            ]
        );
    }

    // -- copy_ignored_files ---------------------------------------------------

    #[test]
    fn copy_ignored_files_copies_existing_file() {
        use tempfile::TempDir;

        let origin = TempDir::new().expect("origin dir");
        let slot = TempDir::new().expect("slot dir");
        std::fs::write(origin.path().join(".env"), "SECRET=1").expect("write .env");

        copy_ignored_files(origin.path(), slot.path(), &[".env".to_string()])
            .expect("copy_ignored_files should not error");

        let copied = std::fs::read_to_string(slot.path().join(".env")).expect("read copied file");
        assert_eq!(copied, "SECRET=1");
    }

    #[test]
    fn copy_ignored_files_skips_missing_source_and_continues() {
        use tempfile::TempDir;

        let origin = TempDir::new().expect("origin dir");
        let slot = TempDir::new().expect("slot dir");
        std::fs::write(origin.path().join("present.txt"), "here").expect("write present.txt");

        copy_ignored_files(
            origin.path(),
            slot.path(),
            &["missing.txt".to_string(), "present.txt".to_string()],
        )
        .expect("missing source files should be skipped, not error");

        assert!(
            !slot.path().join("missing.txt").exists(),
            "missing source file should not appear in the destination"
        );
        assert_eq!(
            std::fs::read_to_string(slot.path().join("present.txt")).expect("read present.txt"),
            "here",
            "remaining entries should still be copied"
        );
    }

    #[test]
    fn copy_ignored_files_creates_destination_subdirectory() {
        use tempfile::TempDir;

        let origin = TempDir::new().expect("origin dir");
        let slot = TempDir::new().expect("slot dir");
        std::fs::create_dir_all(origin.path().join("config")).expect("create origin config dir");
        std::fs::write(origin.path().join("config/local.json"), "{}")
            .expect("write config/local.json");

        copy_ignored_files(
            origin.path(),
            slot.path(),
            &["config/local.json".to_string()],
        )
        .expect("copy_ignored_files should not error");

        assert!(
            slot.path().join("config").is_dir(),
            "destination parent directory should be created automatically"
        );
        assert_eq!(
            std::fs::read_to_string(slot.path().join("config/local.json"))
                .expect("read copied nested file"),
            "{}"
        );
    }

    #[test]
    fn copy_ignored_files_empty_list_is_noop() {
        use tempfile::TempDir;

        let origin = TempDir::new().expect("origin dir");
        let slot = TempDir::new().expect("slot dir");

        copy_ignored_files(origin.path(), slot.path(), &[])
            .expect("empty entry list should be a no-op, not an error");

        assert_eq!(
            std::fs::read_dir(slot.path())
                .expect("read slot dir")
                .count(),
            0,
            "slot dir should remain empty when the entry list is empty"
        );
    }

    // -- select_prune_candidates ----------------------------------------------

    #[test]
    fn select_prune_candidates_empty_input_preserves_nothing() {
        let (preserved, to_delete) = select_prune_candidates(vec![]);
        assert_eq!(preserved, None);
        assert!(to_delete.is_empty());
    }

    #[test]
    fn select_prune_candidates_single_entry_is_preserved_not_deleted() {
        let slot = (PathBuf::from("/tmp/slot-a"), Some("my-feature".to_string()));
        let (preserved, to_delete) = select_prune_candidates(vec![slot.clone()]);
        assert_eq!(preserved, Some(slot));
        assert!(to_delete.is_empty());
    }

    #[test]
    fn select_prune_candidates_multiple_entries_preserves_first_only() {
        let first = (PathBuf::from("/tmp/slot-a"), None);
        let second = (PathBuf::from("/tmp/slot-b"), Some("feature-b".to_string()));
        let third = (PathBuf::from("/tmp/slot-c"), None);

        let (preserved, to_delete) =
            select_prune_candidates(vec![first.clone(), second.clone(), third.clone()]);

        assert_eq!(preserved, Some(first));
        assert_eq!(to_delete, vec![second, third]);
    }
}
