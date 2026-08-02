//! Integration tests for `bs list` / `bs ls`.
//!
//! All tests use [`common::GitEnv`] to run in a fully isolated Docker-backed
//! git environment.  Host `~/.bonsai` is never touched.

mod common;

use bonsai::worktree::tilde_path;
use common::GitEnv;

/// Strip ANSI CSI escape sequences (e.g. color codes from `owo-colors`) from
/// a line so column positions can be computed on the visible text only.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next(); // consume '['
            for nc in chars.by_ref() {
                if nc.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

// ── 4.1: empty pool ───────────────────────────────────────────────────────────

/// `bs list` with no pool directory prints a friendly message and exits 0.
#[tokio::test]
async fn list_no_pool_prints_friendly_message() {
    let env = GitEnv::new().await;

    // No `bs get` has been run — pool dir does not exist yet.
    let out = env.bs().arg("list").output().expect("spawn bs list");

    assert!(
        out.status.success(),
        "bs list should exit 0 when pool is empty\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No worktrees"),
        "expected a friendly 'No worktrees' message, got: {stdout:?}"
    );
}

/// `bs ls` alias behaves identically to `bs list`.
#[tokio::test]
async fn ls_alias_behaves_like_list() {
    let env = GitEnv::new().await;

    let list_out = env.bs().arg("list").output().expect("spawn bs list");
    let ls_out = env.bs().arg("ls").output().expect("spawn bs ls");

    assert!(list_out.status.success());
    assert!(ls_out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&list_out.stdout),
        String::from_utf8_lossy(&ls_out.stdout),
        "`bs list` and `bs ls` should produce identical output"
    );
}

// ── 4.2: one slot, path/branch/current + badge, no stats column ──────────────

/// After `bs get` creates a slot, `bs list` shows its path and the
/// `available` status badge (no stats column).
#[tokio::test]
async fn list_one_slot_shows_path_and_badge() {
    let env = GitEnv::new().await;

    let slot = env.run_get();

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(
        out.status.success(),
        "bs list should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("available"),
        "bs list must print a status badge for a clean unlocked slot, got: {stdout:?}"
    );

    let slot_name = slot.file_name().unwrap().to_str().unwrap();
    assert!(
        stdout.contains(slot_name),
        "output should contain the slot directory name '{slot_name}', got: {stdout:?}"
    );
}

// ── 4.4: branch display, no stats column ─────────────────────────────────────

/// After `bs get` provisions a slot in detached HEAD, attaching a branch and
/// adding an untracked file should show the branch name and the `in use`
/// badge in `bs list` output, but never a stats column (that detail now
/// lives in `bs status`).
#[tokio::test]
async fn list_shows_branch_and_badge_but_no_stats_column() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    // Attach a named branch to the slot (host git is fine for this).
    let attach = common::host_git(&slot, &["checkout", "-b", "feature/my-work"]);
    assert!(
        attach.status.success(),
        "git checkout -b failed: {}",
        String::from_utf8_lossy(&attach.stderr)
    );

    // Add an untracked file — this must NOT show up as a stats icon anymore.
    std::fs::write(slot.join("untracked.txt"), "hello").expect("write untracked file");

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(
        out.status.success(),
        "bs list should exit 0\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("feature/my-work"),
        "expected branch name 'feature/my-work' in output, got: {stdout:?}"
    );
    assert!(
        stdout.contains("in use"),
        "an untracked file should classify the slot as 'in use', got: {stdout:?}"
    );
    assert!(
        !stdout.contains("?1") && !stdout.contains('\u{00b1}') && !stdout.contains('\u{2699}'),
        "bs list must never show stat icons, got: {stdout:?}"
    );
}

/// A dirty, unlocked slot is displayed as `in use`; a locked slot is
/// displayed as `locked` regardless of dirtiness — only the badge changes,
/// never a stats column.
#[tokio::test]
async fn list_dirty_slot_shows_in_use_badge_no_stats() {
    let env = GitEnv::new().await;

    let slot = env.run_get();
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(
        out.status.success(),
        "bs list should still exit 0 for a dirty slot\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("in use"),
        "bs list must print the 'in use' badge for a dirty slot, got: {stdout:?}"
    );
    assert!(
        !stdout.contains("?1") && !stdout.contains('\u{00b1}') && !stdout.contains('\u{2699}'),
        "bs list must never show stat icons, got: {stdout:?}"
    );
}

/// A locked slot is classified `locked` in `bs list`, taking priority over
/// dirtiness.
#[tokio::test]
async fn list_locked_slot_shows_locked_badge() {
    let env = GitEnv::new().await;

    let slot = env.run_get();
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let lock_out = env
        .bs()
        .args(["lock", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("locked"),
        "bs list must print the 'locked' badge for a locked slot, got: {stdout:?}"
    );
    assert!(
        !stdout.contains("in use"),
        "a locked slot must not also show 'in use', got: {stdout:?}"
    );
}

// ── column alignment across badges of different lengths ─────────────────────

/// The worktree path column must start at the same character position on
/// every line, regardless of whether that line's badge is `available` (9
/// chars) or `locked`/`in use` (6 chars each).
#[tokio::test]
async fn list_path_column_aligned_across_badge_lengths() {
    let env = GitEnv::new().await;

    let available_slot = env.run_get();
    let locked_slot = env.run_get();

    let lock_out = env
        .bs()
        .args(["lock", locked_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let out = env.bs().arg("list").output().expect("spawn bs list");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    let available_path = tilde_path(&available_slot);
    let locked_path = tilde_path(&locked_slot);

    let mut available_col = None;
    let mut locked_col = None;

    for line in stdout.lines() {
        let plain = strip_ansi(line);
        if let Some(idx) = plain.find(&available_path) {
            available_col = Some(plain[..idx].chars().count());
        }
        if let Some(idx) = plain.find(&locked_path) {
            locked_col = Some(plain[..idx].chars().count());
        }
    }

    let available_col =
        available_col.expect("expected to find the 'available' slot's path in the output");
    let locked_col = locked_col.expect("expected to find the 'locked' slot's path in the output");

    assert_eq!(
        available_col, locked_col,
        "path column should start at the same character position regardless of badge \
         length, got: {stdout:?}"
    );
}

/// The path column alignment must hold even when the `▶` current-slot prefix
/// is present on one of the rows, alongside slots with differing badge
/// lengths.
#[tokio::test]
async fn list_path_column_aligned_with_current_slot_prefix() {
    let env = GitEnv::new().await;

    let available_slot = env.run_get();
    let locked_slot = env.run_get();

    let lock_out = env
        .bs()
        .args(["lock", locked_slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    // Run `bs list` from inside the locked slot so its row gets the `▶` prefix.
    let out = env
        .bs_from(&locked_slot)
        .arg("list")
        .output()
        .expect("spawn bs list");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains('▶'),
        "expected the current-slot marker in output, got: {stdout:?}"
    );

    let available_path = tilde_path(&available_slot);
    let locked_path = tilde_path(&locked_slot);

    let mut available_col = None;
    let mut locked_col = None;

    for line in stdout.lines() {
        let plain = strip_ansi(line);
        if let Some(idx) = plain.find(&available_path) {
            available_col = Some(plain[..idx].chars().count());
        }
        if let Some(idx) = plain.find(&locked_path) {
            locked_col = Some(plain[..idx].chars().count());
        }
    }

    let available_col =
        available_col.expect("expected to find the 'available' slot's path in the output");
    let locked_col =
        locked_col.expect("expected to find the 'locked'/current slot's path in the output");

    assert_eq!(
        available_col, locked_col,
        "path column should be aligned even when the ▶ current-slot prefix is present, \
         got: {stdout:?}"
    );
}

// ── early-return short-circuit: lsof is skipped for locked/dirty slots ──────

/// `bs list` must not invoke `lsof` for a **locked** slot: with a `PATH` that
/// only exposes `git` (no `lsof` binary reachable), `bs list` must still
/// succeed and report `locked` for a locked slot.
#[tokio::test]
async fn list_locked_slot_skips_lsof() {
    let env = GitEnv::new().await;
    let slot = env.run_get();

    let lock_out = env
        .bs()
        .args(["lock", slot.to_str().unwrap()])
        .output()
        .expect("spawn bs lock");
    assert!(lock_out.status.success());

    let git_only_path = common::git_only_path_dir();

    let out = env
        .bs()
        .arg("list")
        .env("PATH", git_only_path.path())
        .output()
        .expect("spawn bs list");

    assert!(
        out.status.success(),
        "bs list must succeed for a locked slot without lsof on PATH\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("locked"),
        "expected the locked badge without needing lsof, got: {stdout:?}"
    );
}

/// `bs list` must not invoke `lsof` for a **dirty, unlocked** slot: with a
/// `PATH` that only exposes `git`, `bs list` must still succeed and report
/// `in use` for a dirty slot.
#[tokio::test]
async fn list_dirty_slot_skips_lsof() {
    let env = GitEnv::new().await;
    let slot = env.run_get();
    std::fs::write(slot.join("dirty.txt"), "dirty").expect("write dirty file");

    let git_only_path = common::git_only_path_dir();

    let out = env
        .bs()
        .arg("list")
        .env("PATH", git_only_path.path())
        .output()
        .expect("spawn bs list");

    assert!(
        out.status.success(),
        "bs list must succeed for a dirty slot without lsof on PATH\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("in use"),
        "expected the in-use badge without needing lsof, got: {stdout:?}"
    );
}

/// A clean, unlocked slot DOES require `lsof` to distinguish `available` from
/// `in use`: with a `PATH` that only exposes `git`, `bs list` must fail
/// naming `lsof` as the missing dependency.
#[tokio::test]
async fn list_clean_unlocked_slot_requires_lsof() {
    let env = GitEnv::new().await;
    let _slot = env.run_get();

    let git_only_path = common::git_only_path_dir();

    let out = env
        .bs()
        .arg("list")
        .env("PATH", git_only_path.path())
        .output()
        .expect("spawn bs list");

    assert!(
        !out.status.success(),
        "bs list must fail for a clean, unlocked slot without lsof on PATH"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("lsof"),
        "stderr should name 'lsof' as the missing dependency, got: {stderr:?}"
    );
}
