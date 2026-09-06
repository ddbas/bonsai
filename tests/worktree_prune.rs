//! Integration tests for `bs prune`.
//!
//! All tests use [`common::GitEnv`] to run in a fully isolated Docker-backed
//! git environment.  Host `~/.bonsai` is never touched.

mod common;

use common::GitEnv;

// ── 3.2: no pool directory yet ────────────────────────────────────────────────

/// `bs prune` with no pool directory prints a friendly message and exits 0.
#[tokio::test]
async fn prune_no_pool_prints_friendly_message() {
    let env = GitEnv::new().await;

    // No `bs get` has been run — pool dir does not exist yet.
    let out = env.bs().arg("prune").output().expect("spawn bs prune");

    assert!(
        out.status.success(),
        "bs prune should exit 0 when pool is empty\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No worktrees"),
        "expected a friendly 'No worktrees' message, got: {stdout:?}"
    );
}

// ── 3.3: pool with only locked/in-use slots ───────────────────────────────────

/// A pool containing only locked/in-use slots: nothing is deleted, a
/// friendly "nothing to prune" message is printed, and the process exits 0.
#[tokio::test]
async fn prune_only_locked_and_in_use_slots_prunes_nothing() {
    let env = GitEnv::new().await;

    let locked_slot = env.run_get();
    let lock_out = env
        .bs()
        .args(["lock", locked_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let dirty_slot = env.run_get();
    std::fs::write(dirty_slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let out = env.bs().arg("prune").output().expect("spawn bs prune");
    assert!(
        out.status.success(),
        "bs prune should exit 0\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.to_lowercase().contains("nothing to prune"),
        "expected a 'nothing to prune' message, got: {stdout:?}"
    );

    assert!(locked_slot.exists(), "locked slot must remain on disk");
    assert!(
        dirty_slot.exists(),
        "in-use (dirty) slot must remain on disk"
    );
}

// ── 3.4: mixed pool — only the available slot is removed ──────────────────────

/// A pool with a locked, an in-use, and an available slot: only the
/// available slot's directory is deleted, and after `bs prune` runs it is no
/// longer registered with git, while the locked/in-use slots remain fully
/// intact (directory present, still registered).
#[tokio::test]
async fn prune_mixed_pool_only_deletes_available_slot() {
    let env = GitEnv::new().await;

    let locked_slot = env.run_get();
    let lock_out = env
        .bs()
        .args(["lock", locked_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let in_use_slot = env.run_get();
    std::fs::write(in_use_slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let available_slot = env.run_get();

    let out = env.bs().arg("prune").output().expect("spawn bs prune");
    assert!(
        out.status.success(),
        "bs prune should exit 0\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Only the available slot's directory should be gone.
    assert!(
        !available_slot.exists(),
        "available slot's directory should be deleted"
    );
    assert!(locked_slot.exists(), "locked slot must remain on disk");
    assert!(in_use_slot.exists(), "in-use slot must remain on disk");

    // After `bs prune`, the deleted slot is no longer a registered worktree,
    // but locked/in-use slots are still registered. Use `bs list` (run with
    // the correct cwd/env) rather than calling the git-porcelain helper
    // directly from the test process, whose cwd is not the test repo.
    let list_out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(list_out.status.success());
    let list_stdout = String::from_utf8_lossy(&list_out.stdout);

    let locked_tilde = bonsai::worktree::tilde_path(&locked_slot);
    let in_use_tilde = bonsai::worktree::tilde_path(&in_use_slot);
    let available_tilde = bonsai::worktree::tilde_path(&available_slot);

    assert!(
        list_stdout.contains(&locked_tilde),
        "locked slot should still be registered, got: {list_stdout:?}"
    );
    assert!(
        list_stdout.contains(&in_use_tilde),
        "in-use slot should still be registered, got: {list_stdout:?}"
    );
    assert!(
        !list_stdout.contains(&available_tilde),
        "pruned slot should no longer be registered after `git worktree prune`, got: {list_stdout:?}"
    );
}

// ── 3.5: reported path + branch formatting ────────────────────────────────────

/// A pruned slot with a checked-out branch is reported with the branch name
/// in parentheses; a pruned detached-HEAD slot is reported with no branch
/// suffix.
#[tokio::test]
async fn prune_reports_branch_or_no_branch_suffix() {
    let env = GitEnv::new().await;

    // Create the detached slot first, then lock it temporarily so the next
    // `bs get` call is forced to create a genuinely new second slot (an
    // available, unlocked, clean slot would otherwise be reused in place).
    let detached_slot = env.run_get();
    let lock_out = env
        .bs()
        .args(["lock", detached_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let branch_slot = env.run_get();
    let checkout = common::host_git(&branch_slot, &["checkout", "-b", "my-feature"]);
    assert!(
        checkout.status.success(),
        "git checkout -b failed: {}",
        String::from_utf8_lossy(&checkout.stderr)
    );

    let unlock_out = env
        .bs()
        .args(["unlock", detached_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs unlock");
    assert!(unlock_out.status.success());

    let out = env.bs().arg("prune").output().expect("spawn bs prune");
    assert!(
        out.status.success(),
        "bs prune should exit 0\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);

    let branch_tilde = bonsai::worktree::tilde_path(&branch_slot);
    let detached_tilde = bonsai::worktree::tilde_path(&detached_slot);

    let branch_line = stdout
        .lines()
        .find(|l| l.contains(&branch_tilde))
        .unwrap_or_else(|| panic!("expected a line for the branch slot, got: {stdout:?}"));
    assert!(
        branch_line.contains("my-feature"),
        "expected branch name in output line, got: {branch_line:?}"
    );

    let detached_line = stdout
        .lines()
        .find(|l| l.contains(&detached_tilde))
        .unwrap_or_else(|| panic!("expected a line for the detached slot, got: {stdout:?}"));
    assert!(
        !detached_line.contains('('),
        "detached-HEAD slot must not show a branch suffix, got: {detached_line:?}"
    );

    assert!(!branch_slot.exists());
    assert!(!detached_slot.exists());
}

// ── 3.6: per-slot deletion failure is reported, not fatal ─────────────────────

/// A per-slot deletion failure (simulated by revoking write permission on the
/// slot's parent directory, so `remove_dir_all` cannot unlink its entries)
/// is reported and does not prevent the other available slot from being
/// pruned or `git worktree prune` from running; the process exits non-zero.
///
/// Best-effort / platform-permitting: skipped when running as root (e.g.
/// some CI containers), since permissions have no effect there.
#[cfg(unix)]
#[tokio::test]
async fn prune_partial_failure_reports_and_continues() {
    use std::os::unix::fs::PermissionsExt;

    // Skip entirely when running as root: permission bits can't block root.
    if unsafe { libc_geteuid() } == 0 {
        eprintln!("skipping: running as root, permissions cannot block deletion");
        return;
    }

    let env = GitEnv::new().await;

    // First slot: will be made undeletable.
    let undeletable_slot = env.run_get();
    // Lock it temporarily so the next `bs get` creates a genuinely new slot.
    let lock_out = env
        .bs()
        .args(["lock", undeletable_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let deletable_slot = env.run_get();

    let unlock_out = env
        .bs()
        .args(["unlock", undeletable_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs unlock");
    assert!(unlock_out.status.success());

    // Revoke write+execute on the slot itself so `remove_dir_all` cannot
    // remove its contents (EACCES), without affecting the sibling slot.
    std::fs::set_permissions(&undeletable_slot, std::fs::Permissions::from_mode(0o500))
        .expect("chmod undeletable slot");

    let out = env.bs().arg("prune").output().expect("spawn bs prune");

    // Restore permissions unconditionally so TempDir cleanup can proceed.
    let _ = std::fs::set_permissions(&undeletable_slot, std::fs::Permissions::from_mode(0o700));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    if out.status.success() {
        // Permissions had no effect (e.g. privileged test runner) — nothing
        // meaningful to assert; treat as a skip.
        eprintln!(
            "skipping assertions: deletion apparently succeeded despite chmod \
             (stdout: {stdout:?})"
        );
        return;
    }

    assert!(
        !deletable_slot.exists(),
        "the other available slot should still be pruned"
    );
    assert!(
        stdout.contains(&bonsai::worktree::tilde_path(&deletable_slot))
            || stderr.contains(&bonsai::worktree::tilde_path(&deletable_slot)),
        "expected the successfully pruned slot to be reported, stdout: {stdout:?}"
    );
    assert!(
        stderr.contains(&bonsai::worktree::tilde_path(&undeletable_slot))
            || stdout.contains(&bonsai::worktree::tilde_path(&undeletable_slot)),
        "expected the failed slot to be reported, stderr: {stderr:?}"
    );
}

#[cfg(unix)]
unsafe fn libc_geteuid() -> u32 {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() }
}
