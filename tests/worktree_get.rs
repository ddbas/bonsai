//! Integration tests for `bs get` (and the default-command behaviour).
//!
//! Every test uses [`common::GitEnv`], which spins up a Docker container
//! with a pinned `alpine/git` image.  All git operations (init, commit,
//! worktree manipulation) run inside that container so the host machine's
//! git configuration, environment variables, and `~/.bonsai` directory are
//! never touched.

mod common;

use std::path::PathBuf as _;

use common::{GitEnv, host_git, path_from_output, worktree_head};

// ── lsof missing: hard error ──────────────────────────────────────────────────

/// When `lsof` is not on PATH and the pool has a slot to scan, `bs get` must
/// exit non-zero and print an actionable error message that names `lsof` as
/// the missing dependency.
#[tokio::test]
async fn get_fails_with_actionable_error_when_lsof_missing() {
    let env = GitEnv::new().await;

    // Create the first slot so the pool is non-empty; subsequent calls must
    // scan it and therefore invoke `lsof`.
    let _ = env.run_get();

    // Build a fake PATH dir that contains git (symlinked) but not lsof.
    let fake_bin = tempfile::TempDir::new().expect("fake bin dir");
    let git_location = which_binary("git").expect("git must be on PATH for this test");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&git_location, fake_bin.path().join("git"))
        .expect("symlink git into fake bin dir");

    let out = env
        .bs()
        .arg("get")
        .env("PATH", fake_bin.path())
        .output()
        .expect("spawn bs get");

    assert!(
        !out.status.success(),
        "bs get should exit non-zero when lsof is missing"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("lsof"),
        "stderr should name 'lsof' as the missing dependency, got: {stderr:?}"
    );
}

/// Resolve the absolute path of a binary on PATH using the host `which`.
fn which_binary(name: &str) -> Option<std::path::PathBuf> {
    let out = std::process::Command::new("which")
        .arg(name)
        .output()
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(s))
        }
    } else {
        None
    }
}

// ── 7.1: repo_slug uses main repo, not the slot name ─────────────────────────

#[tokio::test]
async fn slug_derived_from_main_repo_not_slot() {
    let env = GitEnv::new().await;
    let slot1 = env.run_get();

    // Dirty slot1 so the next call is forced to create a new slot.
    std::fs::write(slot1.join("dirty.txt"), "dirty").unwrap();

    // Run `bs get` from *inside* slot1 (a linked worktree).
    let out = env
        .bs_from(&slot1)
        .arg("get")
        .output()
        .expect("spawn bs from linked worktree");
    assert!(
        out.status.success(),
        "bs get from linked worktree failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot2 = path_from_output(&out.stdout);
    let slug_dir = env.slug_dir();

    // The new slot should sit under the *main repo's* slug directory, not a
    // directory derived from the slot's own name.
    assert!(
        slot2.starts_with(&slug_dir),
        "slot from linked worktree should be under slug_dir {slug_dir:?}, \
         got {slot2:?}"
    );
}

// ── 7.4 / 8.6: dirty slot skipped → new slot created ────────────────────────

#[tokio::test]
async fn dirty_slot_skipped_new_slot_created() {
    let env = GitEnv::new().await;
    let slot1 = env.run_get();

    // Untracked file makes `git status --porcelain` non-empty → slot is dirty.
    std::fs::write(slot1.join("dirty.txt"), "dirty").unwrap();

    let slot2 = env.run_get();
    assert_ne!(slot1, slot2, "should return a fresh slot");
    assert!(slot2.exists(), "new slot must exist on disk");
    assert!(slot1.exists(), "dirty slot must not be removed");
}

// ── 7.5 / 8.7: locked slot skipped → new slot created ───────────────────────

#[tokio::test]
async fn locked_slot_skipped_new_slot_created() {
    let env = GitEnv::new().await;
    let slot1 = env.run_get();

    // Lock via host git (worktree is registered in the host repo).
    host_git(
        &env.repo_path,
        &["worktree", "lock", &slot1.to_string_lossy()],
    );

    let slot2 = env.run_get();
    assert_ne!(slot1, slot2, "should skip the locked slot");
    assert!(slot2.exists());

    // Unlock for clean teardown.
    let _ = host_git(
        &env.repo_path,
        &["worktree", "unlock", &slot1.to_string_lossy()],
    );
}

// ── 7.6: first clean unlocked slot is returned ───────────────────────────────

#[tokio::test]
async fn first_clean_unlocked_slot_returned() {
    let env = GitEnv::new().await;

    // slot1 → dirty
    let slot1 = env.run_get();
    std::fs::write(slot1.join("dirty.txt"), "dirty").unwrap();

    // slot2 → clean
    let slot2 = env.run_get();
    assert_ne!(slot1, slot2);

    // Third call: slot1 dirty, slot2 clean → slot2 reused.
    let slot3 = env.run_get();
    assert_eq!(slot2, slot3, "clean slot2 should be reused");
}

// ── 7.7 / 8.3: empty pool → new UUID slot created ────────────────────────────

#[tokio::test]
async fn empty_pool_creates_new_slot() {
    let env = GitEnv::new().await;
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

#[tokio::test]
async fn pool_dirs_created_on_first_run() {
    let env = GitEnv::new().await;

    let pre: Vec<_> = std::fs::read_dir(&env.bonsai_path).unwrap().collect();
    assert!(
        pre.is_empty(),
        "bonsai_path should be empty before first run"
    );

    let slot = env.run_get();
    assert!(slot.exists());
    assert!(env.slug_dir().is_dir());
}

// ── 8.2: pool dir creation is idempotent ─────────────────────────────────────

#[tokio::test]
async fn pool_dirs_idempotent() {
    let env = GitEnv::new().await;
    env.run_get();
    // Second call must not error even though the directories already exist.
    env.run_get();
}

// ── 8.4: existing clean slot is reused ───────────────────────────────────────

#[tokio::test]
async fn existing_clean_slot_reused() {
    let env = GitEnv::new().await;
    let slot1 = env.run_get();
    let slot2 = env.run_get();

    assert_eq!(slot1, slot2, "same slot should be returned on second call");
    assert_eq!(env.slots().len(), 1, "only one slot should exist");
}

// ── 8.5: slot reset to current HEAD after a new commit ───────────────────────

#[tokio::test]
async fn slot_reset_to_new_head() {
    let env = GitEnv::new().await;

    let slot = env.run_get();
    let old_head = env.head_sha();
    assert_eq!(worktree_head(&slot), old_head);

    // Advance HEAD via the container (isolated from host git config).
    env.make_commit("second.txt", "v2").await;
    let new_head = env.head_sha();
    assert_ne!(old_head, new_head);

    // Second `bs get` should reset the existing clean slot to the new HEAD.
    let slot2 = env.run_get();
    assert_eq!(slot, slot2, "same slot should be reused");
    assert_eq!(
        worktree_head(&slot),
        new_head,
        "slot HEAD should be updated to the new commit"
    );
}

// ── 8.8: stale registration pruned ───────────────────────────────────────────

#[tokio::test]
async fn stale_registration_pruned() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    assert!(slot.exists());

    // Manually delete the slot directory to create a stale registration.
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

#[tokio::test]
async fn called_from_linked_worktree_uses_main_repo_slug() {
    let env = GitEnv::new().await;

    let slot1 = env.run_get();
    let slug_dir = env.slug_dir();

    // Dirty slot1 so the next call creates slot2.
    std::fs::write(slot1.join("linked_test.txt"), "x").unwrap();

    let out = env
        .bs_from(&slot1)
        .arg("get")
        .output()
        .expect("spawn bs from linked worktree");
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

#[tokio::test]
async fn no_subcommand_behaves_like_bs_get() {
    let env = GitEnv::new().await;

    let out_noarg = env.bs().output().expect("spawn bs");
    assert!(out_noarg.status.success(), "bs with no args should exit 0");
    let path_noarg = path_from_output(&out_noarg.stdout);

    // The slot from the first call is now clean; `bs get` reuses it.
    let out_get = env.bs().arg("get").output().expect("spawn bs get");
    assert!(out_get.status.success());
    let path_get = path_from_output(&out_get.stdout);

    assert_eq!(
        path_noarg, path_get,
        "`bs` and `bs get` should return the same slot"
    );
}

// ── emoji in stdout ───────────────────────────────────────────────────────────

#[tokio::test]
async fn get_output_contains_tree_emoji() {
    let env = GitEnv::new().await;
    let out = env.bs().arg("get").output().expect("spawn bs get");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains('🌳'),
        "stdout should contain the 🌳 emoji; got: {stdout:?}"
    );
}
