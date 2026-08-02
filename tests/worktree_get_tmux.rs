//! Integration tests for `bs get --tmux-session` / `--no-attach`.
//!
//! These tests exercise real `tmux` session creation/reuse against a
//! throwaway tmux server (a unique `-L <socket>` per test, selected via the
//! `BONSAI_TMUX_SOCKET` environment variable that `src/tmux.rs` recognises)
//! so the developer's/CI runner's real tmux server is never touched, and
//! parallel test runs never collide.
//!
//! Every test skips (rather than fails) when `tmux` is not found on `PATH`,
//! matching the pattern used elsewhere in this codebase for other
//! environment-dependent checks (e.g. `lsof`).
//!
//! All tests pass `--no-attach`: `attach-session`/`switch-client` require an
//! interactive controlling terminal, which is not available for a
//! subprocess spawned via `Command::output()`.

mod common;

use std::process::Command;

use common::GitEnv;

/// Returns `true` if `tmux` is reachable on `PATH`.
fn tmux_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A throwaway tmux server, isolated via a unique `-L <socket>` name.
/// Kills the server on drop so tests never leak background tmux servers.
struct TmuxSocket {
    name: String,
}

impl TmuxSocket {
    fn new(label: &str) -> Self {
        TmuxSocket {
            name: format!("bonsai-test-{label}-{}", uuid::Uuid::new_v4()),
        }
    }

    fn has_session(&self, session: &str) -> bool {
        Command::new("tmux")
            .args(["-L", &self.name, "has-session", "-t", session])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn pane_path(&self, session: &str) -> String {
        let out = Command::new("tmux")
            .args([
                "-L",
                &self.name,
                "display-message",
                "-p",
                "-t",
                session,
                "#{pane_current_path}",
            ])
            .output()
            .expect("spawn tmux display-message");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }
}

impl Drop for TmuxSocket {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.name, "kill-server"])
            .output();
    }
}

/// The first line of `bs get`'s stdout — the `🌳 <path> [(branch)]` line —
/// as consumed by [`common::path_from_output`], which is not multi-line
/// aware and would otherwise swallow the following `tmux session: …` line.
fn first_line(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

/// Extract the session name printed by `bs get --tmux-session` from stdout
/// (the `<emoji>  tmux session: <name>` line).
fn session_name_from_output(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    text.lines()
        .find_map(|line| line.split_once("tmux session: "))
        .map(|(_, name)| name.trim().to_string())
        .unwrap_or_else(|| panic!("no tmux session line found in stdout:\n{text}"))
}

// ── creation ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_tmux_session_creates_default_named_session() {
    if !tmux_available() {
        eprintln!("skipping: tmux not found on PATH");
        return;
    }
    let env = GitEnv::new().await;
    let socket = TmuxSocket::new("default-name");

    let out = env
        .bs()
        .args(["get", "--tmux-session", "--no-attach"])
        .env("BONSAI_TMUX_SOCKET", &socket.name)
        .output()
        .expect("spawn bs get --tmux-session --no-attach");

    assert!(
        out.status.success(),
        "bs get --tmux-session --no-attach failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let session_name = session_name_from_output(&out.stdout);
    // Default naming convention: 🌳 <repo-name> (detached) — no branch flags
    // were passed, so branch-display must be the literal `detached`.
    assert!(
        session_name.contains("(detached)"),
        "expected default session name to end in (detached), got: {session_name}"
    );
    assert!(
        socket.has_session(&session_name),
        "expected tmux session `{session_name}` to exist on socket {}",
        socket.name
    );
}

#[tokio::test]
async fn get_tmux_session_session_rooted_at_slot_path() {
    if !tmux_available() {
        eprintln!("skipping: tmux not found on PATH");
        return;
    }
    let env = GitEnv::new().await;
    let socket = TmuxSocket::new("cwd");

    let out = env
        .bs()
        .args(["get", "--tmux-session", "--no-attach"])
        .env("BONSAI_TMUX_SOCKET", &socket.name)
        .output()
        .expect("spawn bs get --tmux-session --no-attach");
    assert!(out.status.success());

    let session_name = session_name_from_output(&out.stdout);
    let slot_path = common::path_from_output(first_line(&out.stdout).as_bytes());

    let pane_path = socket.pane_path(&session_name);
    let pane_canonical = std::path::Path::new(&pane_path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&pane_path));
    let slot_canonical = slot_path
        .canonicalize()
        .unwrap_or_else(|_| slot_path.clone());
    assert_eq!(pane_canonical, slot_canonical);
}

#[tokio::test]
async fn get_tmux_session_default_name_reflects_branch() {
    if !tmux_available() {
        eprintln!("skipping: tmux not found on PATH");
        return;
    }
    let env = GitEnv::new().await;
    let socket = TmuxSocket::new("branch-name");

    let out = env
        .bs()
        .args(["get", "-b", "my-feature", "--tmux-session", "--no-attach"])
        .env("BONSAI_TMUX_SOCKET", &socket.name)
        .output()
        .expect("spawn bs get -b my-feature --tmux-session --no-attach");
    assert!(out.status.success());

    let session_name = session_name_from_output(&out.stdout);
    assert!(
        session_name.contains("(my-feature)"),
        "expected session name to contain (my-feature), got: {session_name}"
    );
}

// ── reuse ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_tmux_session_rerun_reuses_existing_session() {
    if !tmux_available() {
        eprintln!("skipping: tmux not found on PATH");
        return;
    }
    let env = GitEnv::new().await;
    let socket = TmuxSocket::new("reuse");

    let first = env
        .bs()
        .args(["get", "--tmux-session", "--no-attach"])
        .env("BONSAI_TMUX_SOCKET", &socket.name)
        .output()
        .expect("spawn first bs get --tmux-session --no-attach");
    assert!(first.status.success());
    let first_name = session_name_from_output(&first.stdout);

    let second = env
        .bs()
        .args(["get", "--tmux-session", "--no-attach"])
        .env("BONSAI_TMUX_SOCKET", &socket.name)
        .output()
        .expect("spawn second bs get --tmux-session --no-attach");
    assert!(
        second.status.success(),
        "rerunning bs get --tmux-session must not fail when the session already exists\nstderr: {}",
        String::from_utf8_lossy(&second.stderr),
    );
    let second_name = session_name_from_output(&second.stdout);

    assert_eq!(first_name, second_name);
    assert!(socket.has_session(&first_name));

    // Exactly one session should exist — no duplicate was created.
    let list = Command::new("tmux")
        .args(["-L", &socket.name, "list-sessions", "-F", "#{session_name}"])
        .output()
        .expect("spawn tmux list-sessions");
    let names: Vec<String> = String::from_utf8_lossy(&list.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    assert_eq!(
        names.len(),
        1,
        "expected exactly one tmux session, got: {names:?}"
    );
}

// ── custom name ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_tmux_session_custom_name_used_verbatim() {
    if !tmux_available() {
        eprintln!("skipping: tmux not found on PATH");
        return;
    }
    let env = GitEnv::new().await;
    let socket = TmuxSocket::new("custom-name");

    let out = env
        .bs()
        .args(["get", "--tmux-session=my-custom-session", "--no-attach"])
        .env("BONSAI_TMUX_SOCKET", &socket.name)
        .output()
        .expect("spawn bs get --tmux-session=my-custom-session --no-attach");

    assert!(out.status.success());
    let session_name = session_name_from_output(&out.stdout);
    assert_eq!(session_name, "my-custom-session");
    assert!(socket.has_session("my-custom-session"));
}

// ── --no-attach ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_tmux_session_no_attach_creates_without_attaching() {
    if !tmux_available() {
        eprintln!("skipping: tmux not found on PATH");
        return;
    }
    let env = GitEnv::new().await;
    let socket = TmuxSocket::new("no-attach");

    // `--no-attach` must never invoke `switch-client`/`attach-session`, so
    // this must complete (and exit 0) even though stdin/stdout are pipes
    // with no controlling terminal — an `attach-session` call would fail or
    // hang in that situation.
    let out = env
        .bs()
        .args(["get", "--tmux-session", "--no-attach"])
        .env("BONSAI_TMUX_SOCKET", &socket.name)
        .output()
        .expect("spawn bs get --tmux-session --no-attach");

    assert!(out.status.success());
    let session_name = session_name_from_output(&out.stdout);
    assert!(socket.has_session(&session_name));
}

#[tokio::test]
async fn get_no_attach_without_tmux_session_fails_at_parse_time() {
    // Pure clap parsing behaviour — no tmux invocation happens, so this
    // test does not need to check for/skip based on tmux availability.
    let env = GitEnv::new().await;

    let out = env
        .bs()
        .args(["get", "--no-attach"])
        .output()
        .expect("spawn bs get --no-attach");

    assert!(
        !out.status.success(),
        "bs get --no-attach (without --tmux-session) must fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("tmux-session") || stderr.contains("tmux_session"),
        "expected usage error to mention --tmux-session, got: {stderr}"
    );
}
