//! Integration tests for `bs lock` and `bs unlock`.
//!
//! All tests use [`common::GitEnv`] to run in a fully isolated Docker-backed
//! git environment.  Host `~/.bonsai` is never touched.

mod common;

use common::GitEnv;

// ── 6.1: bs lock with an explicit path argument ───────────────────────────────

/// `bs lock <path>` locks the slot; `bs list` then shows it as `locked`.
#[tokio::test]
async fn lock_by_explicit_path_marks_slot_locked() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Lock the slot by its absolute path.
    let out = env
        .bs()
        .args(["lock", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(
        out.status.success(),
        "bs lock should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("locked"),
        "bs lock should print a confirmation containing 'locked', got: {stdout:?}"
    );

    // Verify via git porcelain that the slot is now locked.
    let porcelain = common::host_git(&env.repo_path, &["worktree", "list", "--porcelain"]);
    let porcelain_text = String::from_utf8_lossy(&porcelain.stdout);
    assert!(
        porcelain_text.contains("locked"),
        "git worktree list --porcelain should show 'locked' after bs lock, got: {porcelain_text:?}"
    );
}

// ── 6.2: bs unlock restores Available status ─────────────────────────────────

/// Lock a slot then unlock it; the slot returns to `available` in `bs list`.
#[tokio::test]
async fn unlock_restores_available_status() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    let slot_str = slot.to_str().unwrap();

    // Lock.
    let lock_out = env
        .bs()
        .args(["lock", slot_str])
        .output()
        .expect("spawn bs lock");
    assert!(
        lock_out.status.success(),
        "bs lock failed\nstderr: {}",
        String::from_utf8_lossy(&lock_out.stderr)
    );

    // Unlock.
    let unlock_out = env
        .bs()
        .args(["unlock", slot_str])
        .output()
        .expect("spawn bs unlock");
    assert!(
        unlock_out.status.success(),
        "bs unlock should exit 0\nstderr: {}",
        String::from_utf8_lossy(&unlock_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&unlock_out.stdout);
    assert!(
        stdout.contains("unlocked"),
        "bs unlock should print 'unlocked', got: {stdout:?}"
    );

    // The slot should now appear as `available` in `bs status`
    // (`bs list` no longer prints a status badge).
    let status_out = env
        .bs()
        .args(["status", slot_str])
        .output()
        .expect("spawn bs status");
    assert!(status_out.status.success());
    let status_stdout = String::from_utf8_lossy(&status_out.stdout);
    assert!(
        status_stdout.contains("available"),
        "slot should be 'available' after unlock, got: {status_stdout:?}"
    );
    assert!(
        !status_stdout.contains("locked"),
        "slot should not show 'locked' after unlock, got: {status_stdout:?}"
    );
}

// ── 6.3: bs lock --reason stores the reason string ───────────────────────────

/// `bs lock --reason <msg>` forwards the reason to git; `git worktree list
/// --porcelain` then includes the reason text on the `locked` line.
#[tokio::test]
async fn lock_with_reason_stores_reason() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let reason = "reserved for agent build";
    let out = env
        .bs()
        .args(["lock", "--reason", reason, slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock --reason");
    assert!(
        out.status.success(),
        "bs lock --reason should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Git porcelain includes `locked <reason>` when a reason is set.
    let porcelain = common::host_git(&env.repo_path, &["worktree", "list", "--porcelain"]);
    let porcelain_text = String::from_utf8_lossy(&porcelain.stdout);
    assert!(
        porcelain_text.contains(reason),
        "git porcelain should include the reason string '{reason}', got: {porcelain_text:?}"
    );
}

// ── 6.4: default-to-current-slot behaviour ───────────────────────────────────

/// `bs lock` and `bs unlock` with no path argument default to the slot that
/// contains the current working directory.
#[tokio::test]
async fn lock_unlock_defaults_to_current_slot() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Lock from inside the slot (no path argument).
    let lock_out = env
        .bs_from(&slot)
        .arg("lock")
        .output()
        .expect("spawn bs lock (no path)");
    assert!(
        lock_out.status.success(),
        "bs lock with no path should succeed when run from inside a slot\nstderr: {}",
        String::from_utf8_lossy(&lock_out.stderr)
    );

    // Verify locked.
    let porcelain = common::host_git(&env.repo_path, &["worktree", "list", "--porcelain"]);
    let porcelain_text = String::from_utf8_lossy(&porcelain.stdout);
    assert!(
        porcelain_text.contains("locked"),
        "slot should be locked after default bs lock, got: {porcelain_text:?}"
    );

    // Unlock from inside the slot (no path argument).
    let unlock_out = env
        .bs_from(&slot)
        .arg("unlock")
        .output()
        .expect("spawn bs unlock (no path)");
    assert!(
        unlock_out.status.success(),
        "bs unlock with no path should succeed when run from inside a slot\nstderr: {}",
        String::from_utf8_lossy(&unlock_out.stderr)
    );

    // Verify unlocked via `bs status` (badge/classification now lives there).
    // Run from `repo_path` (not from inside `slot`) so the `bs status`
    // process's own CWD doesn't register as an open handle at the slot root.
    let status_out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    let status_stdout = String::from_utf8_lossy(&status_out.stdout);
    assert!(
        status_stdout.contains("available"),
        "slot should be available after default bs unlock, got: {status_stdout:?}"
    );
}

// ── 6.5: bs status shows `locked` classification; locked+dirty stays `locked` ──

/// A locked slot is reported as `locked` (not `in use`) by `bs status`.
/// A locked slot that also has uncommitted changes is still reported as
/// `locked`. `bs list` never shows a badge for either case.
#[tokio::test]
async fn status_shows_locked_for_locked_slot() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Lock the slot.
    let lock_out = env
        .bs()
        .args(["lock", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    // `bs list` should show no badge at all.
    let list_out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(list_out.status.success());
    let list_stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(
        !list_stdout.contains("locked") && !list_stdout.contains("in use"),
        "bs list must never show a status badge, got: {list_stdout:?}"
    );

    // `bs status` should show `locked`, not `in use`.
    let status_out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(status_out.status.success());
    let status_stdout = String::from_utf8_lossy(&status_out.stdout);
    assert!(
        status_stdout.contains("locked"),
        "bs status should show 'locked' for locked slot, got: {status_stdout:?}"
    );
    assert!(
        !status_stdout.contains("in use"),
        "bs status must not show 'in use' for a locked slot, got: {status_stdout:?}"
    );

    // Now dirty the slot (locked + dirty should still show as `locked`).
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let status_out2 = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status (dirty)");
    assert!(status_out2.status.success());
    let status_stdout2 = String::from_utf8_lossy(&status_out2.stdout);
    assert!(
        status_stdout2.contains("locked"),
        "locked+dirty slot should still show 'locked', got: {status_stdout2:?}"
    );
    assert!(
        !status_stdout2.contains("in use"),
        "locked+dirty slot must not show 'in use', got: {status_stdout2:?}"
    );
}
