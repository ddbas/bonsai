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

// ── 4.2: one slot, path/branch/current only ───────────────────────────────────

/// After `bs get` creates a slot, `bs list` shows its path and no status badge
/// or stats column.
#[tokio::test]
async fn list_one_slot_shows_path_only() {
    let env = GitEnv::new().await;

    let slot = env.run_get();

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(
        out.status.success(),
        "bs list should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);

    // No status badges should ever appear now that `bs list` no longer
    // classifies slots.
    assert!(
        !stdout.contains("available") && !stdout.contains("in use") && !stdout.contains("locked"),
        "bs list must not print a status badge, got: {stdout:?}"
    );

    let slot_name = slot.file_name().unwrap().to_str().unwrap();
    assert!(
        stdout.contains(slot_name),
        "output should contain the slot directory name '{slot_name}', got: {stdout:?}"
    );
}

// ── 4.4: branch display, no stats column ─────────────────────────────────────

/// After `bs get` provisions a slot in detached HEAD, attaching a branch and
/// adding an untracked file should show the branch name in `bs list` output
/// but never a stats column (that detail now lives in `bs status`).
#[tokio::test]
async fn list_shows_branch_but_no_stats_column() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Attach a named branch to the slot (host git is fine for this).
    let attach = common::host_git(&slot, &["checkout", "-b", "feature/my-work"]);
    assert!(
        attach.status.success(),
        "git checkout -b failed: {}",
        String::from_utf8_lossy(&attach.stderr)
    );

    // Add an untracked file — this must NOT show up as a stats icon anymore.
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
        !stdout.contains("?1") && !stdout.contains('\u{00b1}') && !stdout.contains('\u{2699}'),
        "bs list must never show stat icons, got: {stdout:?}"
    );
}

/// A dirty, locked, or busy slot is displayed identically to a clean one in
/// `bs list` — only path/branch/current marker, no badge — regardless of the
/// slot's underlying state.
#[tokio::test]
async fn list_dirty_slot_has_no_badge_or_stats() {
    let env = GitEnv::new().await;

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
        !stdout.contains("available") && !stdout.contains("in use") && !stdout.contains("locked"),
        "bs list must not print a status badge for a dirty slot, got: {stdout:?}"
    );
}

// ── 4.1: bs list does not shell out to lsof ──────────────────────────────────

/// `bs list` must not invoke `lsof` for any slot: with a `PATH` that only
/// exposes `git` (no `lsof` binary reachable), `bs list` must still succeed.
/// Before this change, `bs list` called `list_worktrees_status`, which always
/// shells out to `lsof` per slot and would fail with a "lsof not found"
/// error under this same `PATH`.
#[tokio::test]
async fn list_does_not_shell_out_to_lsof() {
    let env = GitEnv::new().await;
    let _slot = env.run_get();

    let git_only_path = common::git_only_path_dir();

    let out = env
        .bs()
        .arg("list")
        .env("PATH", git_only_path.path())
        .output()
        .expect("spawn bs list");

    assert!(
        out.status.success(),
        "bs list must succeed without lsof on PATH (it must not shell out to lsof)\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
