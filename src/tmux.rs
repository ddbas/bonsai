//! tmux session helpers for `bs get --tmux-session`.
//!
//! Mirrors the tmux invocation pattern used by the external `worktree-get`
//! dotfiles script: `has-session` to check for an existing session,
//! `new-session -ds` to create one detached if missing, and
//! `switch-client`/`attach-session` (depending on whether the caller is
//! already inside a tmux client) to attach.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// The literal branch-display label used when no branch was requested.
pub const DETACHED_LABEL: &str = "detached";

/// Build a `tmux` `Command`.
///
/// When the `BONSAI_TMUX_SOCKET` environment variable is set, every
/// invocation is pinned to that socket via `-L <name>` so integration tests
/// can exercise real tmux session creation/reuse/attach logic against a
/// throwaway server instead of the developer's/CI runner's real tmux server.
/// Unset in normal use, this is a no-op and `tmux` talks to the default
/// socket exactly as it would from a plain shell invocation.
fn tmux_cmd() -> Command {
    let mut cmd = Command::new("tmux");
    if let Ok(socket) = std::env::var("BONSAI_TMUX_SOCKET") {
        cmd.args(["-L", &socket]);
    }
    cmd
}

/// Build the default tmux session name following the `worktree-get`
/// naming convention: `🌳 <repo-name> (<branch-display>)`.
pub fn default_session_name(repo_name: &str, branch_display: &str) -> String {
    format!("\u{1f333} {repo_name} ({branch_display})")
}

/// Resolve the tmux session name for `--tmux-session`.
///
/// An empty value (the `--tmux-session` bare-flag sentinel) derives the
/// default name from `repo_name`/`branch_display`; any other value is used
/// verbatim as the session name.
pub fn resolve_session_name(value: &str, repo_name: &str, branch_display: &str) -> String {
    if value.is_empty() {
        default_session_name(repo_name, branch_display)
    } else {
        value.to_string()
    }
}

/// Verify that `tmux` is reachable on `PATH`, returning an actionable error
/// if it is not.
pub fn check_tmux_available() -> Result<()> {
    match tmux_cmd().arg("-V").output() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!(
                "tmux was not found on PATH; install tmux to use --tmux-session, \
                 or omit the flag to skip tmux integration"
            )
        }
        Err(e) => Err(e).context("failed to check for `tmux` on PATH"),
    }
}

/// Return `true` if a tmux session named `name` already exists.
fn has_session(name: &str) -> Result<bool> {
    let output = tmux_cmd()
        .args(["has-session", "-t", name])
        .output()
        .context("failed to spawn `tmux has-session`")?;
    Ok(output.status.success())
}

/// Create a detached tmux session named `name` rooted at `cwd`, unless one
/// with that name already exists (in which case it is reused as-is).
pub fn ensure_session(name: &str, cwd: &Path) -> Result<()> {
    if has_session(name)? {
        tracing::debug!("Reusing existing tmux session {}", name);
        return Ok(());
    }

    let cwd_str = cwd.to_string_lossy();
    tracing::info!("Creating tmux session {} at {}", name, cwd_str);
    let output = tmux_cmd()
        .args(["new-session", "-d", "-s", name, "-c", &cwd_str])
        .output()
        .context("failed to spawn `tmux new-session`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`tmux new-session` failed: {}", stderr.trim());
    }

    Ok(())
}

/// Attach (or switch, when already inside a tmux client) the invoking
/// terminal to the tmux session named `name`.
///
/// Uses `switch-client` when the `TMUX` environment variable is set
/// (indicating the caller is already inside a tmux client, where
/// `attach-session` would fail), and `attach-session` otherwise.
pub fn attach_session(name: &str) -> Result<()> {
    let inside_tmux = std::env::var_os("TMUX").is_some();

    let mut cmd = tmux_cmd();
    if inside_tmux {
        cmd.args(["switch-client", "-t", name]);
    } else {
        cmd.args(["attach-session", "-t", name]);
    }

    let status = cmd
        .status()
        .context("failed to spawn `tmux switch-client`/`tmux attach-session`")?;

    if !status.success() {
        bail!("failed to attach to tmux session `{}`", name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_session_name_matches_worktree_get_convention() {
        assert_eq!(
            default_session_name("bonsai", "my-feature"),
            "\u{1f333} bonsai (my-feature)"
        );
    }

    #[test]
    fn default_session_name_uses_detached_label() {
        assert_eq!(
            default_session_name("bonsai", DETACHED_LABEL),
            "\u{1f333} bonsai (detached)"
        );
    }

    #[test]
    fn resolve_session_name_empty_value_derives_default() {
        assert_eq!(
            resolve_session_name("", "bonsai", "main"),
            default_session_name("bonsai", "main")
        );
    }

    #[test]
    fn resolve_session_name_non_empty_value_used_verbatim() {
        assert_eq!(
            resolve_session_name("my-custom-session", "bonsai", "main"),
            "my-custom-session"
        );
    }
}
