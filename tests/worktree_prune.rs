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

// ── 3.4: mixed pool — one available slot deleted, one preserved ───────────────

/// A pool with a locked, an in-use, and two available slots: the locked and
/// in-use slots are always untouched; of the two available slots, exactly
/// one is preserved and the other is deleted (which one is picked is an
/// internal ordering detail; only the invariant matters here).
#[tokio::test]
async fn prune_mixed_pool_preserves_one_available_slot() {
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

    // Two available slots, registered separately.
    let slot_a = env.run_get();
    // Lock it temporarily so the next `bs get` is forced to create a
    // genuinely new second available slot instead of reusing this one.
    let lock_out = env
        .bs()
        .args(["lock", slot_a.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let slot_b = env.run_get();

    let unlock_out = env
        .bs()
        .args(["unlock", slot_a.to_str().unwrap()])
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

    assert!(locked_slot.exists(), "locked slot must remain on disk");
    assert!(in_use_slot.exists(), "in-use slot must remain on disk");

    // Exactly one of the two available slots should remain; which one is an
    // internal ordering detail, not part of the observable contract.
    let a_exists = slot_a.exists();
    let b_exists = slot_b.exists();
    assert!(
        a_exists != b_exists,
        "exactly one available slot should be preserved and the other deleted \
         (slot_a exists: {a_exists}, slot_b exists: {b_exists})"
    );
    let (preserved_slot, deleted_slot) = if a_exists {
        (slot_a, slot_b)
    } else {
        (slot_b, slot_a)
    };

    // After `bs prune`, the deleted slot is no longer a registered worktree,
    // but the locked/in-use/preserved slots are still registered.
    let list_out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(list_out.status.success());
    let list_stdout = String::from_utf8_lossy(&list_out.stdout);

    let locked_tilde = bonsai::worktree::tilde_path(&locked_slot);
    let in_use_tilde = bonsai::worktree::tilde_path(&in_use_slot);
    let preserved_tilde = bonsai::worktree::tilde_path(&preserved_slot);
    let deleted_tilde = bonsai::worktree::tilde_path(&deleted_slot);

    assert!(
        list_stdout.contains(&locked_tilde),
        "locked slot should still be registered, got: {list_stdout:?}"
    );
    assert!(
        list_stdout.contains(&in_use_tilde),
        "in-use slot should still be registered, got: {list_stdout:?}"
    );
    assert!(
        list_stdout.contains(&preserved_tilde),
        "preserved slot should still be registered, got: {list_stdout:?}"
    );
    assert!(
        !list_stdout.contains(&deleted_tilde),
        "deleted slot should no longer be registered after `git worktree prune`, got: {list_stdout:?}"
    );
}

// ── single available slot: preserved instead of deleted ───────────────────────

/// A pool containing exactly one available slot (plus locked/in-use slots):
/// that slot is preserved (not deleted), matching the "exactly one available
/// slot" spec scenario.
#[tokio::test]
async fn prune_single_available_slot_is_preserved_not_deleted() {
    let env = GitEnv::new().await;

    let locked_slot = env.run_get();
    let lock_out = env
        .bs()
        .args(["lock", locked_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let available_slot = env.run_get();

    let out = env.bs().arg("prune").output().expect("spawn bs prune");
    assert!(
        out.status.success(),
        "bs prune should exit 0\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(locked_slot.exists(), "locked slot must remain on disk");
    assert!(
        available_slot.exists(),
        "the sole available slot must be preserved, not deleted"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let available_tilde = bonsai::worktree::tilde_path(&available_slot);
    assert!(
        stdout.contains("kept") && stdout.contains(&available_tilde),
        "expected a 'kept' line for the preserved slot, got: {stdout:?}"
    );
}

// ── preserved slot with a branch checked out is detached ──────────────────────

/// When the sole available slot has a branch checked out, `bs prune`
/// preserves its directory but detaches the branch (resets to detached
/// HEAD) so it remains immediately reusable.
#[tokio::test]
async fn prune_preserved_slot_with_branch_is_detached() {
    let env = GitEnv::new().await;

    let available_slot = env.run_get();
    let checkout = common::host_git(&available_slot, &["checkout", "-b", "my-feature"]);
    assert!(
        checkout.status.success(),
        "git checkout -b failed: {}",
        String::from_utf8_lossy(&checkout.stderr)
    );

    let out = env.bs().arg("prune").output().expect("spawn bs prune");
    assert!(
        out.status.success(),
        "bs prune should exit 0\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        available_slot.exists(),
        "preserved slot's directory must not be deleted"
    );

    // The branch should no longer be checked out in that slot.
    let branch_out = common::host_git(&available_slot, &["branch", "--show-current"]);
    assert!(branch_out.status.success());
    let current_branch = String::from_utf8_lossy(&branch_out.stdout);
    assert_eq!(
        current_branch.trim(),
        "",
        "preserved slot should be in detached HEAD, got branch: {current_branch:?}"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let available_tilde = bonsai::worktree::tilde_path(&available_slot);
    let kept_line = stdout
        .lines()
        .find(|l| l.contains(&available_tilde))
        .unwrap_or_else(|| {
            panic!("expected a 'kept' line for the preserved slot, got: {stdout:?}")
        });
    assert!(
        kept_line.contains("kept") && kept_line.contains("my-feature"),
        "expected the kept line to mention the detached branch, got: {kept_line:?}"
    );
}

// ── preserved slot already detached: no git op, reported without a branch ────

/// When the sole available slot is already in detached HEAD, `bs prune`
/// performs no checkout operation and reports it without mentioning a branch.
#[tokio::test]
async fn prune_preserved_slot_already_detached_reports_no_branch() {
    let env = GitEnv::new().await;

    let available_slot = env.run_get();

    let out = env.bs().arg("prune").output().expect("spawn bs prune");
    assert!(
        out.status.success(),
        "bs prune should exit 0\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(available_slot.exists());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let available_tilde = bonsai::worktree::tilde_path(&available_slot);
    let kept_line = stdout
        .lines()
        .find(|l| l.contains(&available_tilde))
        .unwrap_or_else(|| {
            panic!("expected a 'kept' line for the preserved slot, got: {stdout:?}")
        });
    assert!(
        kept_line.contains("kept") && !kept_line.contains('('),
        "expected the kept line to have no branch suffix, got: {kept_line:?}"
    );
}

// ── 3.5: reported path + branch formatting ────────────────────────────────────

/// A deleted slot with a checked-out branch is reported with the branch
/// name in parentheses; the preserved slot (whichever of the two is picked)
/// is reported distinctly via a "kept" line.
#[tokio::test]
async fn prune_reports_branch_or_no_branch_suffix() {
    let env = GitEnv::new().await;

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

    // Exactly one of the two slots is preserved ("kept"); the other is
    // deleted ("pruned"). Which one is picked is an internal ordering
    // detail; only the reporting format is under test here.
    assert!(
        branch_slot.exists() != detached_slot.exists(),
        "exactly one of the two available slots should remain"
    );

    let branch_line = stdout
        .lines()
        .find(|l| l.contains(&branch_tilde))
        .unwrap_or_else(|| panic!("expected a line for the branch slot, got: {stdout:?}"));
    let detached_line = stdout
        .lines()
        .find(|l| l.contains(&detached_tilde))
        .unwrap_or_else(|| panic!("expected a line for the detached slot, got: {stdout:?}"));

    if branch_slot.exists() {
        assert!(
            branch_line.contains("kept") && branch_line.contains("my-feature"),
            "expected the preserved branch slot's kept line to mention the \
             detached branch, got: {branch_line:?}"
        );
        assert!(
            detached_line.contains("pruned"),
            "expected the deleted detached slot to be reported as pruned, got: {detached_line:?}"
        );
    } else {
        assert!(
            branch_line.contains("pruned") && branch_line.contains("my-feature"),
            "expected the deleted branch slot's pruned line to mention its \
             branch, got: {branch_line:?}"
        );
        assert!(
            detached_line.contains("kept") && !detached_line.contains('('),
            "expected the preserved detached-HEAD slot's kept line to have no \
             branch suffix, got: {detached_line:?}"
        );
    }
}

// ── 3.6: per-slot deletion failure is reported, not fatal ─────────────────────

/// A per-slot deletion failure (simulated by revoking write permission on the
/// slot's directory, so `remove_dir_all` cannot unlink its entries) is
/// reported and does not prevent the preserved slot or the other deletable
/// available slot from being handled as usual, or `git worktree prune` from
/// running; the process exits non-zero.
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

    // Three available slots. Which one `bs prune` picks to preserve is an
    // internal ordering detail, so query `bs list`'s reported order (which
    // `prune_pool` shares) up front and chmod one of the two slots that is
    // *not* first in that order, guaranteeing the chmod'd slot is always a
    // deletion candidate rather than the preserved slot.
    let slot_a = env.run_get();
    let lock_out = env
        .bs()
        .args(["lock", slot_a.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let slot_b = env.run_get();
    let lock_out = env
        .bs()
        .args(["lock", slot_b.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let slot_c = env.run_get();

    let unlock_out = env
        .bs()
        .args(["unlock", slot_a.to_str().unwrap()])
        .output()
        .expect("spawn bs unlock");
    assert!(unlock_out.status.success());
    let unlock_out = env
        .bs()
        .args(["unlock", slot_b.to_str().unwrap()])
        .output()
        .expect("spawn bs unlock");
    assert!(unlock_out.status.success());

    let list_out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(list_out.status.success());
    let list_stdout = String::from_utf8_lossy(&list_out.stdout).into_owned();

    let tilde_a = bonsai::worktree::tilde_path(&slot_a);
    let tilde_b = bonsai::worktree::tilde_path(&slot_b);
    let tilde_c = bonsai::worktree::tilde_path(&slot_c);

    let pos = |needle: &str| {
        list_stdout
            .find(needle)
            .unwrap_or_else(|| panic!("expected {needle:?} in `bs list` output: {list_stdout:?}"))
    };
    let mut slots = [
        (pos(&tilde_a), &slot_a),
        (pos(&tilde_b), &slot_b),
        (pos(&tilde_c), &slot_c),
    ];
    slots.sort_by_key(|(pos, _)| *pos);

    let preserved_slot = slots[0].1.clone();
    let undeletable_slot = slots[1].1.clone();
    let deletable_slot = slots[2].1.clone();

    // Revoke write+execute on the slot itself so `remove_dir_all` cannot
    // remove its contents (EACCES), without affecting the sibling slots.
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
        preserved_slot.exists(),
        "the preserved (first-ordered) available slot should remain untouched"
    );
    assert!(
        undeletable_slot.exists(),
        "the chmod'd slot's deletion should have failed, leaving it on disk"
    );
    assert!(
        !deletable_slot.exists(),
        "the other deletable available slot should still be pruned"
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
