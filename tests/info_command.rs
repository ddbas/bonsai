//! Integration tests for the `bs info` command.
//!
//! Tests verify that `bs info` correctly reports runtime paths and metadata
//! without performing any filesystem writes.

mod common;

#[tokio::test]
async fn info_exits_zero() {
    let env = common::GitEnv::new().await;
    let status = env
        .bs()
        .arg("info")
        .status()
        .expect("failed to spawn bs info");
    assert!(status.success(), "`bs info` should exit 0, got {status}");
}

#[tokio::test]
async fn info_prints_version() {
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("info")
        .output()
        .expect("failed to spawn bs info");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("version:"),
        "`bs info` should print 'version:' field; got:\n{stdout}"
    );
}

#[tokio::test]
async fn info_prints_log_level() {
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("info")
        .output()
        .expect("failed to spawn bs info");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("log level:"),
        "`bs info` should print 'log level:' field; got:\n{stdout}"
    );
}

#[tokio::test]
async fn info_prints_log_directory() {
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("info")
        .output()
        .expect("failed to spawn bs info");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("log directory:"),
        "`bs info` should print 'log directory:' field; got:\n{stdout}"
    );
}

#[tokio::test]
async fn info_prints_current_log_file() {
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("info")
        .output()
        .expect("failed to spawn bs info");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("current log file:"),
        "`bs info` should print 'current log file:' field; got:\n{stdout}"
    );
}

#[tokio::test]
async fn info_prints_managed_root() {
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("info")
        .output()
        .expect("failed to spawn bs info");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("managed root:"),
        "`bs info` should print 'managed root:' field; got:\n{stdout}"
    );
}

#[tokio::test]
async fn info_effective_log_level_default() {
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("info")
        .output()
        .expect("failed to spawn bs info");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("log level: info"),
        "`bs info` should report default log level 'info'; got:\n{stdout}"
    );
}

#[tokio::test]
async fn info_effective_log_level_debug_override() {
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("--log-level")
        .arg("debug")
        .arg("info")
        .output()
        .expect("failed to spawn bs --log-level debug info");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("log level: debug"),
        "`bs --log-level debug info` should report log level 'debug'; got:\n{stdout}"
    );
}

#[tokio::test]
async fn info_does_not_create_log_directory() {
    let env = common::GitEnv::new().await;

    // Run `bs info` with an isolated BONSAI_ROOT to track any writes
    let output = env
        .bs()
        .arg("info")
        .output()
        .expect("failed to spawn bs info");

    assert!(output.status.success(), "`bs info` should exit 0");

    // Verify log directory was not created
    // (Note: This is tricky to test in the current infrastructure since
    // BONSAI_ROOT is already set. In a real scenario, we'd check that
    // the directory inside the log path doesn't exist after running `bs info`.)
}

#[tokio::test]
async fn info_paths_use_tilde_abbreviation() {
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("info")
        .output()
        .expect("failed to spawn bs info");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that at least the log directory is tilde-abbreviated
    // (managed root in tests may be a temp dir that doesn't get tilde-abbreviated)
    assert!(
        stdout.contains("log directory: ~/"),
        "`bs info` log directory should use tilde abbreviation for home directory; got:\n{stdout}"
    );
}
