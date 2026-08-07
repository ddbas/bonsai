//! Integration tests for the `bonsai.copy` git-ignored-file copy feature.
//!
//! Every test uses [`common::GitEnv`] (see `tests/worktree_get.rs` for the
//! rationale): git setup/config runs inside an isolated container, while the
//! `bs` binary runs on the host against the shared bind-mounted repo.

mod common;

use common::GitEnv;

/// A file listed in `bonsai.copy` and present in the origin worktree ends up
/// in a freshly created pool slot.
#[tokio::test]
async fn new_slot_receives_configured_copy() {
    let env = GitEnv::new().await;
    env.git(&["config", "--add", "bonsai.copy", ".env"]).await;

    // `.env` is untracked/ignored in the origin worktree; write it directly
    // on the host since it does not need to be committed.
    std::fs::write(env.repo_path.join(".env"), "SECRET=123").expect("write .env");

    let slot = env.run_get();

    let copied = std::fs::read_to_string(slot.join(".env")).expect("read copied .env in slot");
    assert_eq!(copied, "SECRET=123");
}

/// The same configured copy also applies when `bs get` reuses an existing
/// available slot rather than creating a new one.
#[tokio::test]
async fn reused_slot_receives_configured_copy() {
    let env = GitEnv::new().await;
    env.git(&["config", "--add", "bonsai.copy", ".env"]).await;

    // First call creates a slot with no `.env` present yet (file doesn't
    // exist in the origin worktree at this point).
    let slot1 = env.run_get();
    assert!(
        !slot1.join(".env").exists(),
        "no .env in origin worktree yet, so nothing should be copied"
    );

    // Now the origin worktree gets an `.env` file, and the slot is reused
    // (still clean, unlocked) on the next call.
    std::fs::write(env.repo_path.join(".env"), "SECRET=456").expect("write .env");
    let slot2 = env.run_get();

    assert_eq!(slot1, slot2, "the same slot should be reused");
    let copied = std::fs::read_to_string(slot2.join(".env")).expect("read copied .env in slot");
    assert_eq!(copied, "SECRET=456");
}

/// A nonexistent file listed in `bonsai.copy` does not cause `bs get` to
/// fail; other configured files are still copied.
#[tokio::test]
async fn missing_entry_does_not_fail_get_and_others_still_copied() {
    let env = GitEnv::new().await;
    env.git(&["config", "--add", "bonsai.copy", ".env"]).await;
    env.git(&["config", "--add", "bonsai.copy", ".env.missing"])
        .await;

    std::fs::write(env.repo_path.join(".env"), "SECRET=789").expect("write .env");
    // Intentionally do not create `.env.missing`.

    let out = env.bs().arg("get").output().expect("spawn bs get");
    assert!(
        out.status.success(),
        "bs get should succeed even when a bonsai.copy entry is missing\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let slot = common::path_from_output(&out.stdout);
    assert!(
        slot.join(".env").exists(),
        "the present entry should still be copied"
    );
    assert!(
        !slot.join(".env.missing").exists(),
        "the missing entry should simply be absent from the slot"
    );
}

/// With `bonsai.copy` unset, `bs get` behaves exactly as before (regression
/// guard): it succeeds and does not create any unexpected files in the slot.
#[tokio::test]
async fn unset_bonsai_copy_is_a_regression_noop() {
    let env = GitEnv::new().await;

    let slot = env.run_get();

    assert!(slot.exists());
    assert!(
        slot.join("README.md").exists(),
        "the tracked README.md from init should still be present"
    );
}

/// A `bonsai.copy` entry with a nested relative path is copied into the slot
/// with any necessary parent directories created.
#[tokio::test]
async fn nested_relative_path_entry_creates_parent_dirs() {
    let env = GitEnv::new().await;
    env.git(&["config", "--add", "bonsai.copy", "config/local.json"])
        .await;

    std::fs::create_dir_all(env.repo_path.join("config")).expect("create origin config dir");
    std::fs::write(env.repo_path.join("config/local.json"), "{\"k\":1}")
        .expect("write config/local.json");

    let slot = env.run_get();

    assert!(
        slot.join("config").is_dir(),
        "config/ should be created in the slot"
    );
    let copied =
        std::fs::read_to_string(slot.join("config/local.json")).expect("read copied nested file");
    assert_eq!(copied, "{\"k\":1}");
}
