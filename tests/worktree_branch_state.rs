//! E2E tests for branch-state-based slot availability (`bs get`, `bs list`,
//! `bs status`), using a bare `origin` inside the `GitEnv` bind mount.

mod common;

use common::GitEnv;

/// Create `feat` with one commit and push it with upstream tracking, leaving
/// the main repo on `main`.
async fn pushed_feature(env: &GitEnv) {
    env.git(&["checkout", "-q", "-b", "feat"]).await;
    env.make_commit("f.txt", "f").await;
    env.git(&["push", "-u", "origin", "feat"]).await;
    env.git(&["checkout", "-q", "main"]).await;
}

async fn delete_remote_feat(env: &GitEnv) {
    env.git(&["push", "origin", "--delete", "feat"]).await;
    env.fix_permissions().await;
}

#[tokio::test]
async fn fresh_and_detached_at_merged_are_available() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    let slot = env.run_get();
    assert_eq!(env.status_word(&slot), "available");
    let slot2 = env.run_get();
    assert_eq!(slot, slot2, "available slot should be reused");
}

#[tokio::test]
async fn fast_forward_merged_branch_is_available_even_with_remote() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    pushed_feature(&env).await;
    env.git(&["merge", "--ff-only", "feat"]).await;
    env.git(&["push", "origin", "main"]).await;
    env.fix_permissions().await;
    let slot = env.run_get_branch("feat");
    assert_eq!(env.status_word(&slot), "available");
}

#[tokio::test]
async fn squash_merged_remote_gone_is_available() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    pushed_feature(&env).await;
    env.git(&["merge", "--squash", "feat"]).await;
    env.git(&["commit", "-q", "-m", "squash"]).await;
    env.git(&["push", "origin", "main"]).await;
    delete_remote_feat(&env).await;
    // main moves on after the squash merge.
    env.make_commit("later.txt", "later").await;
    env.git(&["push", "origin", "main"]).await;
    let slot = env.run_get_branch("feat");
    assert_eq!(env.status_word(&slot), "available");
}

#[tokio::test]
async fn rebase_merged_remote_gone_is_available() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    pushed_feature(&env).await;
    env.make_commit("other.txt", "o").await;
    env.git(&["cherry-pick", "feat"]).await;
    env.git(&["push", "origin", "main"]).await;
    delete_remote_feat(&env).await;
    let slot = env.run_get_branch("feat");
    assert_eq!(env.status_word(&slot), "available");
}

#[tokio::test]
async fn merged_but_remote_present_is_in_use() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    pushed_feature(&env).await;
    env.git(&["merge", "--squash", "feat"]).await;
    env.git(&["commit", "-q", "-m", "squash"]).await;
    env.git(&["push", "origin", "main"]).await;
    let slot = env.run_get_branch("feat");
    assert_eq!(env.status_word(&slot), "in use");
}

#[tokio::test]
async fn pushed_unmerged_is_in_use() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    pushed_feature(&env).await;
    delete_remote_feat(&env).await;
    // Remote deleted without merging.
    let slot = env.run_get_branch("feat");
    assert_eq!(env.status_word(&slot), "in use");
}

#[tokio::test]
async fn never_pushed_is_in_use_and_not_reused() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    env.git(&["checkout", "-q", "-b", "feat"]).await;
    env.make_commit("f.txt", "f").await;
    env.git(&["checkout", "-q", "main"]).await;
    let slot = env.run_get_branch("feat");
    assert_eq!(env.status_word(&slot), "in use");
    let slot2 = env.run_get();
    assert_ne!(slot, slot2, "slot with unmerged work must not be reused");
}

#[tokio::test]
async fn detached_with_own_commits_is_in_use() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    let slot = env.run_get();
    let out = common::host_git(
        &slot,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=T",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "own",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(env.status_word(&slot), "in use");
}

#[tokio::test]
async fn unresolvable_default_ref_is_in_use() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    // Remote exists but no origin/main or origin/HEAD resolves.
    env.git(&["remote", "set-head", "origin", "-d"]).await;
    env.git(&["update-ref", "-d", "refs/remotes/origin/main"])
        .await;
    let slot = env.run_get();
    assert_eq!(env.status_word(&slot), "in use");
    let out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("default branch could not be determined"),
        "got: {stdout:?}"
    );
}

#[tokio::test]
async fn clean_merged_slot_with_process_is_available_and_reused() {
    let env = GitEnv::new().await;
    env.add_origin().await;
    let slot = env.run_get();
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .current_dir(&slot)
        .spawn()
        .expect("spawn sleep");
    assert_eq!(env.status_word(&slot), "available");
    let slot2 = env.run_get();
    child.kill().ok();
    child.wait().ok();
    assert_eq!(slot, slot2, "slot with a process in it should be reused");
}
