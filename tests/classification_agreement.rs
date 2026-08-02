//! Integration test asserting `bs list`'s badge and `bs status`'s
//! classification agree for the same slot across representative fixtures
//! (locked, dirty, open-process, clean) — guarding against the two call
//! sites (`classify_slot_status` and `slot_status`) drifting apart now that
//! they gather signals differently but both route through the shared
//! `classify()` function.

mod common;

use common::GitEnv;

/// Extract the classification word (`available`/`in use`/`locked`) that both
/// `bs list` and `bs status` print, from either command's stdout.
fn extract_classification(stdout: &str) -> &'static str {
    if stdout.contains("locked") {
        "locked"
    } else if stdout.contains("in use") {
        "in use"
    } else if stdout.contains("available") {
        "available"
    } else {
        panic!("no recognizable classification found in: {stdout:?}");
    }
}

async fn assert_list_and_status_agree(env: &GitEnv, slot: &std::path::Path) {
    let list_out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(
        list_out.status.success(),
        "bs list should succeed\nstderr: {}",
        String::from_utf8_lossy(&list_out.stderr)
    );
    let list_stdout = String::from_utf8_lossy(&list_out.stdout);

    let status_out = env
        .bs()
        .args(["status", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs status");
    assert!(
        status_out.status.success(),
        "bs status should succeed\nstderr: {}",
        String::from_utf8_lossy(&status_out.stderr)
    );
    let status_stdout = String::from_utf8_lossy(&status_out.stdout);

    let list_classification = extract_classification(&list_stdout);
    let status_classification = extract_classification(&status_stdout);

    assert_eq!(
        list_classification, status_classification,
        "bs list and bs status must agree on classification for the same slot\n\
         bs list:\n{list_stdout}\nbs status:\n{status_stdout}"
    );
}

#[tokio::test]
async fn list_and_status_agree_on_clean_slot() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    assert_list_and_status_agree(&env, &slot).await;
}

#[tokio::test]
async fn list_and_status_agree_on_dirty_slot() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");
    assert_list_and_status_agree(&env, &slot).await;
}

#[tokio::test]
async fn list_and_status_agree_on_locked_slot() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    let lock_out = env
        .bs()
        .args(["lock", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());
    assert_list_and_status_agree(&env, &slot).await;
}

#[tokio::test]
async fn list_and_status_agree_on_locked_and_dirty_slot() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");
    let lock_out = env
        .bs()
        .args(["lock", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());
    assert_list_and_status_agree(&env, &slot).await;
}

#[tokio::test]
async fn list_and_status_agree_on_open_process_slot() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let file_path = slot.join("held_open.txt");
    std::fs::write(&file_path, b"data").expect("write");
    let _handle = std::fs::File::open(&file_path).expect("open file");

    assert_list_and_status_agree(&env, &slot).await;
}
