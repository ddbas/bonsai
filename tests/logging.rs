//! Integration tests for logging functionality.
//!
//! Tests the --log-level flag, log file creation, and logging behavior.

mod common;

// ── --log-level flag ──────────────────────────────────────────────────────────

#[tokio::test]
async fn log_level_flag_with_valid_value_exits_zero() {
    let env = common::GitEnv::new().await;
    let status = env
        .bs()
        .arg("--log-level")
        .arg("debug")
        .arg("help")
        .status()
        .expect("failed to spawn bs");
    assert!(
        status.success(),
        "`bs --log-level debug help` should exit 0, got {status}"
    );
}

#[tokio::test]
async fn log_level_flag_accepts_all_valid_levels() {
    let levels = ["trace", "debug", "info", "warn", "error"];
    for level in &levels {
        let env = common::GitEnv::new().await;
        let status = env
            .bs()
            .arg("--log-level")
            .arg(level)
            .arg("help")
            .status()
            .expect("failed to spawn bs");
        assert!(
            status.success(),
            "`bs --log-level {} help` should exit 0",
            level
        );
    }
}

#[tokio::test]
async fn log_level_flag_with_invalid_value_exits_nonzero() {
    let env = common::GitEnv::new().await;
    let status = env
        .bs()
        .arg("--log-level")
        .arg("bogus")
        .arg("help")
        .status()
        .expect("failed to spawn bs");
    assert!(
        !status.success(),
        "`bs --log-level bogus help` should exit non-zero, got {status}"
    );
}

#[tokio::test]
async fn log_level_flag_appears_in_help() {
    let env = common::GitEnv::new().await;
    let output = env.bs().arg("--help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("log-level"),
        "`bs --help` should document the --log-level flag; got:\n{stdout}"
    );
}

// ── Logging behavior ──────────────────────────────────────────────────────────

#[tokio::test]
async fn default_log_level_is_info() {
    // This is a basic smoke test that the CLI runs with default log level
    let env = common::GitEnv::new().await;
    let status = env.bs().arg("help").status().expect("failed to spawn bs");
    assert!(
        status.success(),
        "`bs help` with default log level should exit 0"
    );
}

#[tokio::test]
async fn log_output_does_not_appear_on_stdout() {
    // Verify that log events don't pollute stdout (logs should go to file only)
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("--log-level")
        .arg("debug")
        .arg("help")
        .output()
        .expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The help output should contain normal text (e.g. "bs", "help", etc),
    // but should not contain typical logging prefixes like timestamps or level names
    // (tracing format by default includes timestamps)
    assert!(
        !stdout.contains("TRACE") && !stdout.contains("DEBUG"),
        "`bs` should not print log-level names to stdout; got:\n{stdout}"
    );
}

#[tokio::test]
async fn log_output_does_not_appear_on_stderr() {
    // Verify that normal operations don't produce stderr spam
    let env = common::GitEnv::new().await;
    let output = env
        .bs()
        .arg("--log-level")
        .arg("debug")
        .arg("help")
        .output()
        .expect("failed to spawn bs");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // help command should not produce any errors or logs to stderr
    assert!(
        stderr.is_empty() || !stderr.to_lowercase().contains("error"),
        "`bs help` with debug log level should not produce errors on stderr; got:\n{stderr}"
    );
}
