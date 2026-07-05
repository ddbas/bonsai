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
// `bs` with no arguments now defaults to `bs get`, so it exits 0 and prints
// a worktree path (prefixed with 🌳) to stdout.
//
// Both tests create an isolated temp git repo and a temp BONSAI_ROOT so they
// don't touch ~/.bonsai and don't interfere with the host repo's git index
// (important: these tests also run inside the pre-commit hook).

fn make_temp_git_repo() -> tempfile::TempDir {
    use std::process::Command;
    let repo = tempfile::TempDir::new().unwrap();
    let p = repo.path();
    let run = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(p)
            .status()
            .unwrap();
    };
    run(&["init"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "commit.gpgsign", "false"]);
    std::fs::write(p.join("README.md"), "# test").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "init"]);
    repo
}

#[test]
fn no_args_exits_zero() {
    use tempfile::TempDir;
    let repo = make_temp_git_repo();
    let bonsai = TempDir::new().unwrap();
    let status = bs()
        .current_dir(repo.path())
        .env("BONSAI_ROOT", bonsai.path())
        .status()
        .expect("failed to spawn bs");
    assert!(
        status.success(),
        "`bs` with no arguments should exit 0 (default `get` command), got {status}"
    );
}

#[test]
fn no_args_prints_tree_emoji_and_path_to_stdout() {
    use tempfile::TempDir;
    let repo = make_temp_git_repo();
    let bonsai = TempDir::new().unwrap();
    let output = bs()
        .current_dir(repo.path())
        .env("BONSAI_ROOT", bonsai.path())
        .output()
        .expect("failed to spawn bs");
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
