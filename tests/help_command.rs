//! Integration tests for the `bs help` command.
//!
//! All tests run the binary inside a [`common::GitEnv`] Docker container
//! environment so the host machine's git configuration, filesystem, and
//! home directory are never touched — even if a future `bs help`
//! implementation starts shelling out to git.
mod common;

// ── bs help ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn help_subcommand_exits_zero() {
    let env = common::GitEnv::new().await;
    let status = env.bs().arg("help").status().expect("failed to spawn bs");
    assert!(status.success(), "`bs help` should exit 0, got {status}");
}

#[tokio::test]
async fn help_subcommand_prints_program_name() {
    let env = common::GitEnv::new().await;
    let output = env.bs().arg("help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bs"),
        "`bs help` stdout should contain the program name; got:\n{stdout}"
    );
}

#[tokio::test]
async fn help_subcommand_prints_about_text() {
    let env = common::GitEnv::new().await;
    let output = env.bs().arg("help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bonsai"),
        "`bs help` stdout should contain 'bonsai'; got:\n{stdout}"
    );
}

#[tokio::test]
async fn help_subcommand_lists_help_subcommand() {
    let env = common::GitEnv::new().await;
    let output = env.bs().arg("help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("help"),
        "`bs help` stdout should list the 'help' subcommand; got:\n{stdout}"
    );
}

// ── bs --help ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn flag_help_exits_zero() {
    let env = common::GitEnv::new().await;
    let status = env.bs().arg("--help").status().expect("failed to spawn bs");
    assert!(status.success(), "`bs --help` should exit 0, got {status}");
}

#[tokio::test]
async fn flag_help_prints_usage() {
    let env = common::GitEnv::new().await;
    let output = env.bs().arg("--help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_lowercase().contains("usage"),
        "`bs --help` stdout should contain 'Usage'; got:\n{stdout}"
    );
}

// ── bs (no args) ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_args_exits_zero() {
    let env = common::GitEnv::new().await;
    let status = env.bs().status().expect("failed to spawn bs");
    assert!(
        status.success(),
        "`bs` with no arguments should exit 0 (default `get` command), got {status}"
    );
}

#[tokio::test]
async fn no_args_prints_tree_emoji_and_path_to_stdout() {
    let env = common::GitEnv::new().await;
    let output = env.bs().output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('🌳'),
        "`bs` with no args should print the 🌳 emoji to stdout; got:\n{stdout}"
    );
    assert!(
        !stdout.trim().is_empty(),
        "`bs` with no args should print a non-empty line to stdout"
    );
}
