//! Integration tests for `bs get` (and the default-command behaviour).
//!
//! Each test creates an isolated temporary git repository and a temporary
//! `BONSAI_ROOT` directory so there is no interaction with the developer's
//! real `~/.bonsai` pool or the host repository.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

// ── Test harness ──────────────────────────────────────────────────────────────

/// Holds all temporary state for one test: a fresh git repo and a fresh
/// bonsai root directory (passed to the binary via `BONSAI_ROOT`).
struct TestEnv {
    repo: TempDir,
    bonsai_root: TempDir,
}

impl TestEnv {
    /// Create a new test environment with a git repo containing one commit.
    fn new() -> Self {
        let repo = TempDir::new().expect("temp repo dir");
        let bonsai_root = TempDir::new().expect("temp bonsai root");

        let p = repo.path();
        git(p, &["init"]);
        git(p, &["config", "user.email", "test@example.com"]);
        git(p, &["config", "user.name", "Test"]);
        git(p, &["config", "commit.gpgsign", "false"]);
        std::fs::write(p.join("README.md"), "# test").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "init"]);

        TestEnv { repo, bonsai_root }
    }

    /// A `bs` command builder with CWD=repo and BONSAI_ROOT set.
    fn bs(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_bs"));
        cmd.current_dir(self.repo.path())
            .env("BONSAI_ROOT", self.bonsai_root.path());
        cmd
    }

    /// A `bs` command builder with CWD overridden to `dir` (e.g. a slot).
    fn bs_from(&self, dir: &Path) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_bs"));
        cmd.current_dir(dir)
            .env("BONSAI_ROOT", self.bonsai_root.path());
        cmd
    }

    /// Run `bs get`, assert success, and return the slot path from stdout.
    fn run_get(&self) -> PathBuf {
        let out = self.bs().arg("get").output().expect("failed to spawn bs");
        assert!(
            out.status.success(),
            "bs get failed\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        path_from_output(&out.stdout)
    }

    /// HEAD SHA of the test repo.
    fn head_sha(&self) -> String {
        head_sha(self.repo.path())
    }

    /// Make a new commit in the test repo.
    fn make_commit(&self, filename: &str, content: &str) {
        std::fs::write(self.repo.path().join(filename), content).unwrap();
        git(self.repo.path(), &["add", "."]);
        git(
            self.repo.path(),
            &["commit", "-m", &format!("add {filename}")],
        );
    }

    /// The single slug directory created under `bonsai_root` after the first
    /// `bs get`.  Panics if there isn't exactly one.
    fn slug_dir(&self) -> PathBuf {
        let entries: Vec<_> = std::fs::read_dir(self.bonsai_root.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().canonicalize().unwrap_or_else(|_| e.path()))
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "expected exactly one slug dir under bonsai root, found: {entries:?}"
        );
        entries.into_iter().next().unwrap()
    }

    /// All slot directories under the slug directory (sorted).
    fn slots(&self) -> Vec<PathBuf> {
        let mut v: Vec<_> = std::fs::read_dir(self.slug_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        v.sort();
        v
    }
}

// ── Small helpers ─────────────────────────────────────────────────────────────

/// Run a git command in `dir`; panic on failure.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn git {args:?}: {e}"));
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// Parse the worktree path from `bs get` stdout.
///
/// stdout format: `🌳 /absolute/path/to/slot\n`
fn path_from_output(raw: &[u8]) -> PathBuf {
    let text = String::from_utf8_lossy(raw);
    // Strip the leading tree emoji and any surrounding whitespace.
    let stripped = text.trim().trim_start_matches('🌳').trim();
    PathBuf::from(stripped)
}

/// HEAD SHA of the worktree at `path`.
fn head_sha(path: &Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

// ── 7.1: repo_slug uses main repo, not the slot name ─────────────────────────

#[test]
fn slug_derived_from_main_repo_not_slot() {
    let env = TestEnv::new();
    let slot1 = env.run_get();

    // Make slot1 dirty so a new slot is forced on the next call.
    std::fs::write(slot1.join("dirty.txt"), "dirty").unwrap();

    // Run `bs get` from *inside* slot1 (a linked worktree).
    let out = env
        .bs_from(&slot1)
        .arg("get")
        .output()
        .expect("bs failed from linked worktree");
    assert!(
        out.status.success(),
        "bs get from linked worktree failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot2 = path_from_output(&out.stdout);
    let slug_dir = env.slug_dir();

    // The new slot should be under the same slug directory (main repo's
    // name), not under a directory named after the slot itself.
    assert!(
        slot2.starts_with(&slug_dir),
        "slot from linked worktree should be under slug_dir {slug_dir:?}, got {slot2:?}"
    );
}

// ── 7.4 / 8.6: dirty slot skipped, new slot created ─────────────────────────

#[test]
fn dirty_slot_skipped_new_slot_created() {
    let env = TestEnv::new();
    let slot1 = env.run_get();

    // Make slot1 dirty (untracked file → status --porcelain is non-empty).
    std::fs::write(slot1.join("dirty.txt"), "dirty").unwrap();

    let slot2 = env.run_get();
    assert_ne!(slot1, slot2, "should return a fresh slot");
    assert!(slot2.exists(), "new slot must exist on disk");
    assert!(slot1.exists(), "dirty slot must not be removed");
}

// ── 7.5 / 8.7: locked slot skipped, new slot created ────────────────────────

#[test]
fn locked_slot_skipped_new_slot_created() {
    let env = TestEnv::new();
    let slot1 = env.run_get();

    git(
        env.repo.path(),
        &["worktree", "lock", &slot1.to_string_lossy()],
    );

    let slot2 = env.run_get();
    assert_ne!(slot1, slot2, "should skip the locked slot");
    assert!(slot2.exists());

    // Unlock for clean teardown.
    let _ = Command::new("git")
        .args(["worktree", "unlock", &slot1.to_string_lossy()])
        .current_dir(env.repo.path())
        .status();
}

// ── 7.6: first clean unlocked slot is returned ───────────────────────────────

#[test]
fn first_clean_unlocked_slot_returned() {
    let env = TestEnv::new();

    // Create slot1, then dirty it.
    let slot1 = env.run_get();
    std::fs::write(slot1.join("dirty.txt"), "dirty").unwrap();

    // Create slot2 (clean).
    let slot2 = env.run_get();
    assert_ne!(slot1, slot2);

    // Third call: slot1 is dirty, slot2 is clean → slot2 reused.
    let slot3 = env.run_get();
    assert_eq!(slot2, slot3, "clean slot2 should be reused");
}

// ── 7.7 / 8.3: empty pool → new UUID slot created ────────────────────────────

#[test]
fn empty_pool_creates_new_slot() {
    let env = TestEnv::new();
    let slot = env.run_get();

    assert!(slot.exists(), "slot must exist on disk");

    let name = slot.file_name().unwrap().to_str().unwrap();
    assert_eq!(name.len(), 8, "slot name must be 8 chars, got: {name}");
    assert!(
        name.chars().all(|c| c.is_ascii_hexdigit()),
        "slot name must be hex, got: {name}"
    );
}

// ── 8.1: pool dirs created on first run ──────────────────────────────────────

#[test]
fn pool_dirs_created_on_first_run() {
    let env = TestEnv::new();

    // BONSAI_ROOT exists but is empty before the first run.
    let pre: Vec<_> = std::fs::read_dir(env.bonsai_root.path()).unwrap().collect();
    assert!(
        pre.is_empty(),
        "bonsai_root should be empty before first run"
    );

    let slot = env.run_get();
    assert!(slot.exists(), "slot must exist");

    let slug_dir = env.slug_dir();
    assert!(slug_dir.is_dir(), "slug directory must be created");
}

// ── 8.2: pool dir creation is idempotent ─────────────────────────────────────

#[test]
fn pool_dirs_idempotent() {
    let env = TestEnv::new();
    env.run_get();
    // Second call must not error even though the directories already exist.
    env.run_get();
}

// ── 8.4: existing clean slot is reused ───────────────────────────────────────

#[test]
fn existing_clean_slot_reused() {
    let env = TestEnv::new();
    let slot1 = env.run_get();
    let slot2 = env.run_get();

    assert_eq!(slot1, slot2, "same slot should be returned on second call");
    assert_eq!(env.slots().len(), 1, "only one slot should exist");
}

// ── 8.5: slot reset to current HEAD after a new commit ───────────────────────

#[test]
fn slot_reset_to_new_head() {
    let env = TestEnv::new();

    let slot = env.run_get();
    let old_head = env.head_sha();
    assert_eq!(head_sha(&slot), old_head);

    // Advance HEAD.
    env.make_commit("second.txt", "v2");
    let new_head = env.head_sha();
    assert_ne!(old_head, new_head);

    // Second `bs get` should reset the existing clean slot.
    let slot2 = env.run_get();
    assert_eq!(slot, slot2, "same slot should be reused");
    assert_eq!(
        head_sha(&slot),
        new_head,
        "slot HEAD should be updated to the new commit"
    );
}

// ── 8.8: stale registration pruned ───────────────────────────────────────────

#[test]
fn stale_registration_pruned() {
    let env = TestEnv::new();
    let slot = env.run_get();
    assert!(slot.exists());

    // Delete the slot directory to create a stale git worktree registration.
    std::fs::remove_dir_all(&slot).unwrap();
    assert!(!slot.exists());

    // Next `bs get` must prune the stale entry and create a fresh slot.
    let new_slot = env.run_get();
    assert!(
        new_slot.exists(),
        "a fresh slot should be created after pruning"
    );
}

// ── 8.9: called from a linked worktree ───────────────────────────────────────

#[test]
fn called_from_linked_worktree_uses_main_repo_slug() {
    let env = TestEnv::new();

    let slot1 = env.run_get();
    let slug_dir = env.slug_dir();

    // Dirty slot1 so the next call creates slot2.
    std::fs::write(slot1.join("linked_test.txt"), "x").unwrap();

    let out = env
        .bs_from(&slot1)
        .arg("get")
        .output()
        .expect("bs failed from linked worktree");
    assert!(
        out.status.success(),
        "bs get from linked worktree failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot2 = path_from_output(&out.stdout);
    assert!(
        slot2.starts_with(&slug_dir),
        "slot2 should be under slug_dir {slug_dir:?}, got {slot2:?}"
    );
}

// ── 8.10: `bs` with no subcommand behaves like `bs get` ──────────────────────

#[test]
fn no_subcommand_behaves_like_bs_get() {
    let env = TestEnv::new();

    // Run `bs` (no subcommand).
    let out_noarg = env.bs().output().expect("bs failed");
    assert!(out_noarg.status.success(), "bs with no args should exit 0");
    let path_noarg = path_from_output(&out_noarg.stdout);

    // The slot is now clean; `bs get` should reuse it.
    let out_get = env.bs().arg("get").output().expect("bs get failed");
    assert!(out_get.status.success());
    let path_get = path_from_output(&out_get.stdout);

    assert_eq!(
        path_noarg, path_get,
        "`bs` and `bs get` should return the same slot path"
    );
}

// ── emoji in stdout ───────────────────────────────────────────────────────────

#[test]
fn get_output_contains_tree_emoji() {
    let env = TestEnv::new();
    let out = env.bs().arg("get").output().expect("bs get failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains('🌳'),
        "stdout should contain the 🌳 emoji; got: {stdout:?}"
    );
}
