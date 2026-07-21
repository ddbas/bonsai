//! Integration tests for `bs get` (and the default-command behaviour).
//!
//! Every test uses [`common::GitEnv`], which spins up a Docker container
//! with a pinned `alpine/git` image.  All git operations (init, commit,
//! worktree manipulation) run inside that container so the host machine's
//! git configuration, environment variables, and `~/.bonsai` directory are
//! never touched.

mod common;

use std::path::PathBuf as _;

use common::{GitEnv, branch_from_output, host_git, path_from_output, worktree_head};

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

// ── -b: creates new branch in fresh slot ─────────────────────────────────────

#[tokio::test]
async fn get_b_creates_new_branch() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "-b", "my-feature"])
        .output()
        .expect("spawn bs get -b");
    assert!(
        out.status.success(),
        "bs get -b should succeed\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot = path_from_output(&out.stdout);
    assert!(slot.exists(), "slot must exist on disk");

    // Slot should be on the requested branch, not detached HEAD.
    let branch = current_branch(&slot);
    assert_eq!(branch, "my-feature", "slot should be on branch my-feature");

    // Branch name should appear in stdout.
    let shown = branch_from_output(&out.stdout);
    assert_eq!(
        shown.as_deref(),
        Some("my-feature"),
        "stdout should include the branch name; got: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}

// ── -b: reused slot also ends up on the requested branch ─────────────────────

#[tokio::test]
async fn get_b_reused_slot_gets_branch() {
    let env = GitEnv::new().await;

    // Provision one slot so the pool is non-empty.
    let slot1 = env.run_get();

    // Reuse that slot with -b.
    let out = env
        .bs()
        .args(["get", "-b", "reuse-branch"])
        .output()
        .expect("spawn bs get -b (reuse)");
    assert!(
        out.status.success(),
        "bs get -b on reused slot should succeed\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot2 = path_from_output(&out.stdout);
    assert_eq!(slot1, slot2, "existing clean slot should be reused");
    assert_eq!(current_branch(&slot2), "reuse-branch");
}

// ── -b: fails when branch already exists ─────────────────────────────────────

#[tokio::test]
async fn get_b_fails_if_branch_exists() {
    let env = GitEnv::new().await;

    // Create the branch in the repo (container exec, no checkout).
    env.git(&["branch", "existing-branch"]).await;

    let out = env
        .bs()
        .args(["get", "-b", "existing-branch"])
        .output()
        .expect("spawn bs get -b existing");

    assert!(
        !out.status.success(),
        "bs get -b should fail when branch already exists"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("existing-branch"),
        "stderr should name the conflicting branch; got: {stderr:?}"
    );
}

// ── -B: creates branch when it does not exist ─────────────────────────────────

#[tokio::test]
async fn get_B_creates_fresh_branch() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "-B", "new-branch"])
        .output()
        .expect("spawn bs get -B");
    assert!(
        out.status.success(),
        "bs get -B should succeed for a new branch\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot = path_from_output(&out.stdout);
    assert_eq!(current_branch(&slot), "new-branch");

    let shown = branch_from_output(&out.stdout);
    assert_eq!(shown.as_deref(), Some("new-branch"));
}

// ── -B: resets existing branch to current HEAD ───────────────────────────────

#[tokio::test]
async fn get_B_resets_existing_branch() {
    let env = GitEnv::new().await;

    // Create a branch at the initial commit (before we advance HEAD).
    env.git(&["branch", "to-reset"]).await;

    // Advance HEAD so the branch is now behind.
    env.make_commit("second.txt", "v2").await;
    let new_head = env.head_sha();

    let out = env
        .bs()
        .args(["get", "-B", "to-reset"])
        .output()
        .expect("spawn bs get -B reset");
    assert!(
        out.status.success(),
        "bs get -B should succeed when branch already exists\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot = path_from_output(&out.stdout);
    assert_eq!(current_branch(&slot), "to-reset");
    assert_eq!(
        worktree_head(&slot),
        new_head,
        "branch should be reset to the new HEAD"
    );
}

// ── no flags: slot stays in detached HEAD (regression guard) ─────────────────

#[tokio::test]
async fn get_without_flags_is_detached_head() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // `git symbolic-ref HEAD` exits non-zero for detached HEAD.
    let out = host_git(&slot, &["symbolic-ref", "--short", "HEAD"]);
    assert!(
        !out.status.success(),
        "slot should be in detached HEAD state, but symbolic-ref succeeded: {}",
        String::from_utf8_lossy(&out.stdout).trim()
    );

    // path_from_output should still work (no branch suffix).
    let re_out = env.bs().arg("get").output().expect("spawn");
    assert!(branch_from_output(&re_out.stdout).is_none());
}

// ── -b and -B together are rejected by clap ───────────────────────────────────

#[tokio::test]
async fn get_b_and_B_together_are_rejected() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "-b", "foo", "-B", "bar"])
        .output()
        .expect("spawn bs get -b foo -B bar");

    assert!(
        !out.status.success(),
        "supplying both -b and -B should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "stderr should describe the mutual-exclusion constraint; got: {stderr:?}"
    );
}

/// Regression guard: adding the positional `<branch>` argument must not
/// affect plain `-b`/`-B` usage (no positional branch supplied).
#[tokio::test]
async fn get_b_and_B_alone_are_unaffected_by_positional_arg() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "-b", "solo-branch"])
        .output()
        .expect("spawn bs get -b solo-branch");
    assert!(
        out.status.success(),
        "bs get -b alone should still succeed\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let slot = path_from_output(&out.stdout);
    assert_eq!(current_branch(&slot), "solo-branch");
}

// ── positional branch: checks out an existing, unclaimed branch ────────────

#[tokio::test]
async fn get_positional_branch_checks_out_existing_branch() {
    let env = GitEnv::new().await;

    // Create the branch in the repo without checking it out anywhere.
    env.git(&["branch", "my-existing-branch"]).await;

    let out = env
        .bs()
        .args(["get", "my-existing-branch"])
        .output()
        .expect("spawn bs get <branch>");
    assert!(
        out.status.success(),
        "bs get <branch> should succeed for an existing, unclaimed branch\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot = path_from_output(&out.stdout);
    assert!(slot.exists(), "slot must exist on disk");
    assert_eq!(
        current_branch(&slot),
        "my-existing-branch",
        "slot should be checked out on the existing branch, not detached"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains('🌳'),
        "stdout should contain the 🌳 emoji; got: {stdout:?}"
    );
    let shown = branch_from_output(&out.stdout);
    assert_eq!(
        shown.as_deref(),
        Some("my-existing-branch"),
        "stdout should include the branch name; got: {stdout:?}"
    );
}

// ── positional branch: non-existent branch errors out ───────────────────────

#[tokio::test]
async fn get_positional_branch_nonexistent_errors() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "no-such-branch"])
        .output()
        .expect("spawn bs get <branch>");

    assert!(
        !out.status.success(),
        "bs get <branch> should fail when the branch does not exist"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no-such-branch"),
        "stderr should name the missing branch; got: {stderr:?}"
    );

    // No slot should have been created or reset: any slug directory created
    // under the pool root must contain no slot subdirectories.
    if let Ok(entries) = std::fs::read_dir(env.bonsai_path.as_path()) {
        for slug in entries.filter_map(|e| e.ok()) {
            let slots: Vec<_> = std::fs::read_dir(slug.path())
                .map(|d| d.filter_map(|e| e.ok()).collect())
                .unwrap_or_default();
            assert!(
                slots.is_empty(),
                "no worktree slot should be created for a non-existent branch, got: {slots:?}"
            );
        }
    }
}

// ── positional branch: already checked out in another managed slot ──────────

/// When `<branch>` is already checked out in another bonsai-managed pool slot
/// for this repo, `bs get <branch>` returns that existing slot's path
/// (with branch name) and exits 0, without provisioning or resetting a slot.
#[tokio::test]
async fn get_positional_branch_already_checked_out_in_managed_slot_returns_existing_slot() {
    let env = GitEnv::new().await;

    // Provision a slot and check out `shared-branch` in it via -b.
    let out = env
        .bs()
        .args(["get", "-b", "shared-branch"])
        .output()
        .expect("spawn bs get -b shared-branch");
    assert!(out.status.success(), "initial bs get -b should succeed");
    let claimed_slot = path_from_output(&out.stdout);

    // Dirty the claimed slot so `find_available_slot` would skip it if the
    // branch-ownership pre-check were not in place; this ensures the test
    // exercises the pre-check rather than an incidental slot reuse.
    std::fs::write(claimed_slot.join("dirty.txt"), "dirty").unwrap();

    let out2 = env
        .bs()
        .args(["get", "shared-branch"])
        .output()
        .expect("spawn bs get shared-branch");

    assert!(
        out2.status.success(),
        "bs get <branch> should succeed when the branch is already checked out \
         in a managed slot for this repo\nstderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );

    let returned_slot = path_from_output(&out2.stdout);
    assert_eq!(
        returned_slot, claimed_slot,
        "should return the existing slot that already has shared-branch checked out"
    );

    let shown = branch_from_output(&out2.stdout);
    assert_eq!(
        shown.as_deref(),
        Some("shared-branch"),
        "stdout should include the branch name"
    );

    // The dirtying marker must still be present: the slot must not have been
    // reset or otherwise touched.
    assert!(
        claimed_slot.join("dirty.txt").exists(),
        "the existing slot must not be reset when returned via the pre-check"
    );

    // No second slot should have been created for this repo's pool.
    let slug_dir = env.slug_dir();
    let slot_count = std::fs::read_dir(&slug_dir)
        .map(|d| d.filter_map(|e| e.ok()).count())
        .unwrap_or(0);
    assert_eq!(
        slot_count, 1,
        "no new slot should be created when the branch is already claimed"
    );
}

/// When the already-claimed managed slot has additionally been locked via
/// `bs lock`, `bs get <branch>` must still return that slot's path
/// successfully, and the slot must remain locked afterward.
#[tokio::test]
async fn get_positional_branch_already_checked_out_in_locked_managed_slot_returns_existing_slot() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "-b", "shared-branch"])
        .output()
        .expect("spawn bs get -b shared-branch");
    assert!(out.status.success(), "initial bs get -b should succeed");
    let claimed_slot = path_from_output(&out.stdout);

    let lock_out = env
        .bs()
        .args(["lock", claimed_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(
        lock_out.status.success(),
        "bs lock should succeed\nstderr: {}",
        String::from_utf8_lossy(&lock_out.stderr)
    );

    let out2 = env
        .bs()
        .args(["get", "shared-branch"])
        .output()
        .expect("spawn bs get shared-branch");

    assert!(
        out2.status.success(),
        "bs get <branch> should succeed even when the existing slot is locked\nstderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let returned_slot = path_from_output(&out2.stdout);
    assert_eq!(
        returned_slot, claimed_slot,
        "should return the locked slot that already has shared-branch checked out"
    );

    // The slot must remain locked afterward.
    let porcelain = host_git(&env.repo_path, &["worktree", "list", "--porcelain"]);
    let porcelain_text = String::from_utf8_lossy(&porcelain.stdout);
    assert!(
        porcelain_text.contains("locked"),
        "slot should remain locked after bs get <branch>, got: {porcelain_text:?}"
    );
}

/// Regression test: calling `bs get shared-branch` twice in a row after the
/// branch was initially provisioned via `-b` must both succeed and return
/// the same slot path — neither invocation should error.
#[tokio::test]
async fn get_positional_branch_repeated_calls_return_same_slot() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "-b", "shared-branch"])
        .output()
        .expect("spawn bs get -b shared-branch");
    assert!(out.status.success(), "initial bs get -b should succeed");
    let claimed_slot = path_from_output(&out.stdout);

    let out2 = env
        .bs()
        .args(["get", "shared-branch"])
        .output()
        .expect("spawn bs get shared-branch (first)");
    assert!(
        out2.status.success(),
        "first repeated bs get shared-branch should succeed\nstderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    assert_eq!(path_from_output(&out2.stdout), claimed_slot);

    let out3 = env
        .bs()
        .args(["get", "shared-branch"])
        .output()
        .expect("spawn bs get shared-branch (second)");
    assert!(
        out3.status.success(),
        "second repeated bs get shared-branch should succeed\nstderr: {}",
        String::from_utf8_lossy(&out3.stderr)
    );
    assert_eq!(path_from_output(&out3.stdout), claimed_slot);
}

// ── positional branch: already checked out in an unmanaged worktree ──────────

#[tokio::test]
async fn get_positional_branch_already_checked_out_in_unmanaged_worktree_errors() {
    let env = GitEnv::new().await;

    // Create a worktree OUTSIDE the bonsai pool, on the host, checked out on
    // `unmanaged-branch`.
    let unmanaged_dir = tempfile::TempDir::new().expect("unmanaged worktree dir");
    let unmanaged_path = unmanaged_dir.path().join("wt");
    let add_out = host_git(
        &env.repo_path,
        &[
            "worktree",
            "add",
            "-b",
            "unmanaged-branch",
            unmanaged_path.to_str().unwrap(),
        ],
    );
    assert!(
        add_out.status.success(),
        "failed to create unmanaged worktree: {}",
        String::from_utf8_lossy(&add_out.stderr)
    );

    let out = env
        .bs()
        .args(["get", "unmanaged-branch"])
        .output()
        .expect("spawn bs get unmanaged-branch");

    assert!(
        !out.status.success(),
        "bs get <branch> should fail when the branch is checked out in an unmanaged worktree"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(unmanaged_path.to_str().unwrap()) || stderr.contains("unmanaged-branch"),
        "stderr should name the conflicting unmanaged worktree path or branch; got: {stderr:?}"
    );
}

// ── positional branch vs -b/-B: clap mutual exclusion ───────────────────────

#[tokio::test]
async fn get_positional_branch_conflicts_with_dash_b() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "-b", "foo", "existing-branch"])
        .output()
        .expect("spawn bs get -b foo existing-branch");

    assert!(
        !out.status.success(),
        "positional branch + -b should be rejected by clap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "stderr should describe the mutual-exclusion constraint; got: {stderr:?}"
    );
}

#[tokio::test]
async fn get_positional_branch_conflicts_with_dash_B() {
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "-B", "foo", "existing-branch"])
        .output()
        .expect("spawn bs get -B foo existing-branch");

    assert!(
        !out.status.success(),
        "positional branch + -B should be rejected by clap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "stderr should describe the mutual-exclusion constraint; got: {stderr:?}"
    );
}

// ── Local helpers ──────────────────────────────────────────────────────────

/// Return the short branch name of the worktree at `dir`, or `"HEAD"` for
/// detached HEAD (mirrors `git rev-parse --abbrev-ref HEAD` output).
fn current_branch(dir: &std::path::Path) -> String {
    let out = host_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}
