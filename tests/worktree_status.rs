//! Integration tests for `bs status`.
//!
//! All tests use [`common::GitEnv`] to run in a fully isolated Docker-backed
//! git environment.  Host `~/.bonsai` is never touched.

mod common;

use common::GitEnv;

// ── explicit path invocation ─────────────────────────────────────────────────

/// `bs status <path>` on a clean, unlocked slot reports `available`.
#[tokio::test]
async fn status_explicit_path_reports_available() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");

    assert!(
        out.status.success(),
        "bs status should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("available"),
        "expected 'available' in output, got: {stdout:?}"
    );

    let slot_name = slot.file_name().unwrap().to_str().unwrap();
    assert!(
        stdout.contains(slot_name),
        "output should contain the slot directory name '{slot_name}', got: {stdout:?}"
    );
}

// ── default-to-current-slot invocation ───────────────────────────────────────

/// `bs status` with no arguments, run from inside a managed slot, reports on
/// that slot.
#[tokio::test]
async fn status_defaults_to_current_slot() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let out = env
        .bs_from(&slot)
        .arg("status")
        .output()
        .expect("spawn bs status (no path)");

    assert!(
        out.status.success(),
        "bs status with no path should succeed when run from inside a slot\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    let slot_name = slot.file_name().unwrap().to_str().unwrap();
    assert!(
        stdout.contains(slot_name),
        "output should contain the slot directory name '{slot_name}', got: {stdout:?}"
    );
}

/// `bs status` with no arguments, run from a subdirectory of a managed slot,
/// still reports on the containing slot.
#[tokio::test]
async fn status_defaults_to_current_slot_from_subdirectory() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    let subdir = slot.join("subdir");
    std::fs::create_dir(&subdir).expect("create subdir");

    let out = env
        .bs_from(&subdir)
        .arg("status")
        .output()
        .expect("spawn bs status (no path, subdirectory)");

    assert!(
        out.status.success(),
        "bs status should succeed from a slot subdirectory\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let slot_name = slot.file_name().unwrap().to_str().unwrap();
    assert!(
        stdout.contains(slot_name),
        "output should contain the containing slot's directory name '{slot_name}', got: {stdout:?}"
    );
}

// ── error when CWD is not in a managed slot and no path given ───────────────

/// `bs status` with no arguments, run from outside any managed slot, exits
/// non-zero with an actionable error.
#[tokio::test]
async fn status_no_path_outside_managed_slot_errors() {
    let env = GitEnv::new().await;
    let _ = env.run_get();

    // repo_path is not itself inside the managed pool.
    let out = env
        .bs_from(&env.repo_path)
        .arg("status")
        .output()
        .expect("spawn bs status (no path, outside pool)");

    assert!(
        !out.status.success(),
        "bs status with no path outside a managed slot should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not inside a managed bonsai pool slot"),
        "stderr should explain the CWD is not inside a managed slot, got: {stderr:?}"
    );
}

// ── error for a path outside the pool ────────────────────────────────────────

/// `bs status <path>` where `<path>` exists but is outside the pool exits
/// non-zero with an actionable error.
#[tokio::test]
async fn status_path_outside_pool_errors() {
    let env = GitEnv::new().await;
    let _ = env.run_get();

    let out = env
        .bs()
        .args(["status", env.repo_path.to_str().unwrap()])
        .output()
        .expect("spawn bs status (path outside pool)");

    assert!(
        !out.status.success(),
        "bs status with a path outside the pool should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not a bonsai-managed pool slot"),
        "stderr should explain the path is not a pool slot, got: {stderr:?}"
    );
}

/// `bs status <path>` where `<path>` does not exist on disk exits non-zero
/// naming the missing path.
#[tokio::test]
async fn status_nonexistent_path_errors() {
    let env = GitEnv::new().await;
    let _ = env.run_get();

    let missing = env.bonsai_path.join("does-not-exist-xyz");
    let out = env
        .bs()
        .args(["status", missing.to_str().unwrap()])
        .output()
        .expect("spawn bs status (nonexistent path)");

    assert!(
        !out.status.success(),
        "bs status with a nonexistent path should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not exist"),
        "stderr should mention the path does not exist, got: {stderr:?}"
    );
}

// ── classification priority: locked > in use > available ────────────────────

/// A clean, unlocked slot with no open processes is classified `available`.
#[tokio::test]
async fn status_clean_unlocked_slot_is_available() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("available"), "got: {stdout:?}");
}

/// A dirty, unlocked slot is classified `in use`.
#[tokio::test]
async fn status_dirty_slot_is_in_use() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("in use"), "got: {stdout:?}");
    assert!(!stdout.contains("available"), "got: {stdout:?}");
}

/// A locked slot is classified `locked` even when it is also dirty.
#[tokio::test]
async fn status_locked_and_dirty_slot_is_locked() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let lock_out = env
        .bs()
        .args(["lock", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("locked"), "got: {stdout:?}");
    assert!(!stdout.contains("in use"), "got: {stdout:?}");
}

// ── lock reason display ──────────────────────────────────────────────────────

/// A slot locked with `--reason` shows the reason text in `bs status` output.
#[tokio::test]
async fn status_shows_lock_reason_when_present() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let reason = "reserved for agent build";
    let lock_out = env
        .bs()
        .args(["lock", "--reason", reason, slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock --reason");
    assert!(lock_out.status.success());

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(reason),
        "expected lock reason '{reason}' in output, got: {stdout:?}"
    );
}

/// A slot locked without `--reason` indicates no reason was given.
#[tokio::test]
async fn status_shows_no_reason_when_absent() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let lock_out = env
        .bs()
        .args(["lock", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("locked"),
        "expected 'locked' in output, got: {stdout:?}"
    );
    assert!(
        stdout.to_lowercase().contains("none") || stdout.to_lowercase().contains("no reason"),
        "expected an indication that no reason was given, got: {stdout:?}"
    );
}

// ── itemized uncommitted/untracked file lists ────────────────────────────────

/// A slot with modified and untracked files lists each individually rather
/// than only counts.
#[tokio::test]
async fn status_lists_individual_uncommitted_and_untracked_files() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Modify a tracked file.
    let tracked = common::host_git(&slot, &["ls-files"]);
    let tracked_files = String::from_utf8_lossy(&tracked.stdout);
    let first_tracked = tracked_files
        .lines()
        .next()
        .expect("repo should have at least one tracked file");
    std::fs::write(slot.join(first_tracked), "modified content").expect("modify tracked file");

    // Add an untracked file.
    std::fs::write(slot.join("untracked.txt"), "hello").expect("write untracked file");

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains(first_tracked),
        "expected modified file '{first_tracked}' listed individually, got: {stdout:?}"
    );
    assert!(
        stdout.contains("untracked.txt"),
        "expected untracked file 'untracked.txt' listed individually, got: {stdout:?}"
    );
}

/// A clean slot indicates there are no uncommitted or untracked files.
#[tokio::test]
async fn status_clean_slot_has_no_file_sections() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("uncommitted changes") && !stdout.contains("untracked files"),
        "a clean slot should not print uncommitted/untracked sections, got: {stdout:?}"
    );
}

// ── itemized process list ────────────────────────────────────────────────────

/// A process with an open file descriptor directly at the slot root is
/// listed with its PID and command name, not merely a count.
#[tokio::test]
async fn status_lists_individual_open_process() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Hold a file open directly in the slot root for the duration of the
    // `bs status` call below; `lsof -w +d <slot>` should detect this test
    // process's PID.
    let file_path = slot.join("held_open.txt");
    std::fs::write(&file_path, b"data").expect("write");
    let _handle = std::fs::File::open(&file_path).expect("open file");

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(
        out.status.success(),
        "bs status should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("open processes"),
        "expected an 'open processes' section, got: {stdout:?}"
    );
    let this_pid = std::process::id().to_string();
    assert!(
        stdout.contains(&this_pid),
        "expected this test process's PID '{this_pid}' listed, got: {stdout:?}"
    );
    assert!(stdout.contains("in use"), "got: {stdout:?}");
}

// ── lsof unavailability is a hard error ──────────────────────────────────────

/// When `lsof` cannot be found, `bs status` exits non-zero naming `lsof` as
/// the missing dependency.
#[tokio::test]
async fn status_fails_with_actionable_error_when_lsof_missing() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let git_only_path = common::git_only_path_dir();

    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .env("PATH", git_only_path.path())
        .output()
        .expect("spawn bs status");

    assert!(
        !out.status.success(),
        "bs status should exit non-zero when lsof is missing"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("lsof"),
        "stderr should name 'lsof' as the missing dependency, got: {stderr:?}"
    );
}
