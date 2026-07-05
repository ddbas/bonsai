//! Integration tests for `bs list` / `bs ls`.
//!
//! All tests use [`common::GitEnv`] to run in a fully isolated Docker-backed
//! git environment.  Host `~/.bonsai` is never touched.

mod common;

use common::GitEnv;

// ── 4.1: empty pool ───────────────────────────────────────────────────────────

/// `bs list` with no pool directory prints a friendly message and exits 0.
#[tokio::test]
async fn list_no_pool_prints_friendly_message() {
    let env = GitEnv::new().await;

    // No `bs get` has been run — pool dir does not exist yet.
    let out = env.bs().arg("list").output().expect("spawn bs list");

    assert!(
        out.status.success(),
        "bs list should exit 0 when pool is empty\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No worktrees"),
        "expected a friendly 'No worktrees' message, got: {stdout:?}"
    );
}

/// `bs ls` alias behaves identically to `bs list`.
#[tokio::test]
async fn ls_alias_behaves_like_list() {
    let env = GitEnv::new().await;

    let list_out = env.bs().arg("list").output().expect("spawn bs list");
    let ls_out = env.bs().arg("ls").output().expect("spawn bs ls");

    assert!(list_out.status.success());
    assert!(ls_out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&list_out.stdout),
        String::from_utf8_lossy(&ls_out.stdout),
        "`bs list` and `bs ls` should produce identical output"
    );
}

// ── 4.2: one available slot ───────────────────────────────────────────────────

/// After `bs get` creates a clean slot, `bs list` reports it as `available`.
#[tokio::test]
async fn list_one_available_slot() {
    let env = GitEnv::new().await;

    // Create a slot (clean by default).
    let slot = env.run_get();

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(
        out.status.success(),
        "bs list should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("available"),
        "expected 'available' badge in output, got: {stdout:?}"
    );

    // The slot path should appear with a `~` prefix (BONSAI_ROOT is a TempDir
    // under the real home dir on the test host, so tilde_path should apply).
    let slot_name = slot.file_name().unwrap().to_str().unwrap();
    assert!(
        stdout.contains(slot_name),
        "output should contain the slot directory name '{slot_name}', got: {stdout:?}"
    );
}

// ── 4.4: branch and stats display ───────────────────────────────────────────

/// After `bs get` provisions a slot in detached HEAD, we manually attach a
/// branch to the slot with `host_git`, add an untracked file, and verify:
/// - the branch name appears in `bs list` output
/// - the stats column shows `?1`
#[tokio::test]
async fn list_shows_branch_and_untracked_stats() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Attach a named branch to the slot (host git is fine for this).
    let attach = common::host_git(&slot, &["checkout", "-b", "feature/my-work"]);
    assert!(
        attach.status.success(),
        "git checkout -b failed: {}",
        String::from_utf8_lossy(&attach.stderr)
    );

    // Add an untracked file.
    std::fs::write(slot.join("untracked.txt"), "hello").expect("write untracked file");

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(
        out.status.success(),
        "bs list should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("feature/my-work"),
        "expected branch name 'feature/my-work' in output, got: {stdout:?}"
    );
    assert!(
        stdout.contains("?1"),
        "expected untracked stat '?1' in output, got: {stdout:?}"
    );
}

/// A slot with a branch attached and a clean working tree shows the branch
/// name with an `available` badge and no stat icons.
#[tokio::test]
async fn list_available_slot_shows_branch_no_stats() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Attach a branch without touching any files (slot stays clean).
    let attach = common::host_git(&slot, &["checkout", "-b", "clean-branch"]);
    assert!(
        attach.status.success(),
        "git checkout -b failed: {}",
        String::from_utf8_lossy(&attach.stderr)
    );

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("clean-branch"),
        "expected branch name 'clean-branch' in output, got: {stdout:?}"
    );
    assert!(
        stdout.contains("available"),
        "expected 'available' badge, got: {stdout:?}"
    );
    // No stat icons should be present for a clean slot.
    assert!(
        !stdout.contains("?1") && !stdout.contains('\u{00b1}') && !stdout.contains('\u{2699}'),
        "clean slot should have no stat icons, got: {stdout:?}"
    );
}

/// A slot with an untracked/modified file is reported as `in use`.
#[tokio::test]
async fn list_dirty_slot_shown_as_in_use() {
    let env = GitEnv::new().await;

    // Obtain a slot then dirty it.
    let slot = env.run_get();
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(
        out.status.success(),
        "bs list should still exit 0 for a dirty slot\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("in use"),
        "expected 'in use' badge for dirty slot, got: {stdout:?}"
    );
}
