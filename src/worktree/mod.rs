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
}

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

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            // Flush the previous entry when we hit the next `worktree` line.
            if let Some(path) = cur_path.take() {
                let canonical = path.canonicalize().unwrap_or(path);
                if canonical.starts_with(&pool_canonical) {
                    entries.push(WorktreeEntry {
                        path: canonical,
                        locked: cur_locked,
                    });
                }
            }
            cur_path = Some(PathBuf::from(rest.trim()));
            cur_locked = false;
        } else if line.starts_with("locked") {
            cur_locked = true;
        }
    }

    // Flush the last entry.
    if let Some(path) = cur_path.take() {
        let canonical = path.canonicalize().unwrap_or(path);
        if canonical.starts_with(&pool_canonical) {
            entries.push(WorktreeEntry {
                path: canonical,
                locked: cur_locked,
            });
        }
    }

    Ok(entries)
}

/// Return the availability status of every pool worktree slot.
///
/// Each slot is classified as [`WorktreeStatus::Available`] when it exists on
/// disk, is not locked, and its working tree is clean; otherwise it is
/// [`WorktreeStatus::InUse`].
pub fn list_worktrees_status(pool_dir: &Path) -> Result<Vec<(PathBuf, WorktreeStatus)>> {
    let entries = list_pool_worktrees(pool_dir)?;
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries {
        let status = if entry.locked || !entry.path.exists() {
            WorktreeStatus::InUse
        } else {
            match is_clean(&entry.path) {
                Ok(true) => WorktreeStatus::Available,
                _ => WorktreeStatus::InUse,
            }
        };
        result.push((entry.path, status));
    }
    Ok(result)
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
        if is_clean(&entry.path)? {
            return Ok(Some(entry.path));
        }
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
    /// different combinations of `locked` flag and `is_clean` result.
    /// We test this via a synthetic `WorktreeEntry`-style evaluation rather
    /// than calling `list_worktrees_status` directly (which needs git on the
    /// PATH and a real pool dir).
    #[test]
    fn status_locked_is_in_use() {
        // locked=true should always be InUse regardless of cleanliness
        let locked = true;
        let exists = true;
        let clean = true;
        let status = synthetic_status(locked, exists, clean);
        assert_eq!(status, WorktreeStatus::InUse);
    }

    #[test]
    fn status_nonexistent_is_in_use() {
        let status = synthetic_status(false, false, true);
        assert_eq!(status, WorktreeStatus::InUse);
    }

    #[test]
    fn status_dirty_is_in_use() {
        let status = synthetic_status(false, true, false);
        assert_eq!(status, WorktreeStatus::InUse);
    }

    #[test]
    fn status_clean_unlocked_is_available() {
        let status = synthetic_status(false, true, true);
        assert_eq!(status, WorktreeStatus::Available);
    }

    /// Helper that mirrors the `list_worktrees_status` classification logic
    /// without requiring a live git repository.
    fn synthetic_status(locked: bool, exists: bool, clean: bool) -> WorktreeStatus {
        if locked || !exists {
            WorktreeStatus::InUse
        } else if clean {
            WorktreeStatus::Available
        } else {
            WorktreeStatus::InUse
        }
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
