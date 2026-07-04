/// Integration tests for the `bs help` command.
///
/// These tests exercise the real binary via `std::process::Command`.
/// No Docker or external services are required.
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

#[test]
fn no_args_exits_nonzero() {
    let status = bs().status().expect("failed to spawn bs");
    assert!(
        !status.success(),
        "`bs` with no arguments should exit non-zero (arg_required_else_help), got {status}"
    );
}

#[test]
fn no_args_still_prints_help() {
    let output = bs().output().expect("failed to spawn bs");
    // clap writes the short help to stderr when no subcommand is given.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("bs"),
        "`bs` with no args should print help text; got stdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
