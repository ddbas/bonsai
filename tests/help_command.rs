/// Integration tests for the `bs help` command.
///
/// These tests exercise the real binary via `std::process::Command`.
/// Tests that require an isolated git environment use [`common::GitEnv`].
mod common;

use std::process::Command;

/// Path to the compiled `bs` binary, resolved at compile time by Cargo.
fn bs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bs"))
}

// ── bs help ─────────────────────────────────────────────────────────────────

#[test]
fn help_subcommand_exits_zero() {
    let status = bs().arg("help").status().expect("failed to spawn bs");
    assert!(status.success(), "`bs help` should exit 0, got {status}");
}

#[test]
fn help_subcommand_prints_program_name() {
    let output = bs().arg("help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bs"),
        "`bs help` stdout should contain the program name; got:\n{stdout}"
    );
}

#[test]
fn help_subcommand_prints_about_text() {
    let output = bs().arg("help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bonsai"),
        "`bs help` stdout should contain 'bonsai'; got:\n{stdout}"
    );
}

#[test]
fn help_subcommand_lists_help_subcommand() {
    let output = bs().arg("help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("help"),
        "`bs help` stdout should list the 'help' subcommand; got:\n{stdout}"
    );
}

// ── bs --help ────────────────────────────────────────────────────────────────

#[test]
fn flag_help_exits_zero() {
    let status = bs().arg("--help").status().expect("failed to spawn bs");
    assert!(status.success(), "`bs --help` should exit 0, got {status}");
}

#[test]
fn flag_help_prints_usage() {
    let output = bs().arg("--help").output().expect("failed to spawn bs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_lowercase().contains("usage"),
        "`bs --help` stdout should contain 'Usage'; got:\n{stdout}"
    );
}

// ── bs (no args) ─────────────────────────────────────────────────────────────
// `bs` with no arguments defaults to `bs get`.  These tests use `GitEnv` to
// provide a fully isolated git repo + BONSAI_ROOT (Docker container) so the
// host machine is never touched, even when the tests run inside a git hook.

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
