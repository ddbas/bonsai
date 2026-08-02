//! Criterion benchmark for `bs ls` (`Commands::List`)'s underlying pool-scan
//! logic, tracking the performance SLO defined in
//! `openspec/changes/add-bs-status-command/specs/worktree-list/spec.md`:
//!
//!   - p95 wall-clock latency of the pool scan SHALL be <= 50ms for pools of
//!     up to 50 managed worktree slots (warm filesystem cache).
//!   - The pool-scan cost SHALL NOT scale materially with pool size: p95 at
//!     50 slots SHALL be <= 1.5x p95 at 5 slots.
//!
//! This benchmarks the in-process library call that backs `bs list`
//! (`bonsai::worktree::list_pool_worktrees`), not the full `bs` process
//! spawn, so it isolates the cost this proposal targets (a single
//! `git worktree list --porcelain` invocation with no per-slot `lsof`/
//! `git status` fan-out) from unrelated process-startup overhead. For
//! whole-process latency (including binary startup), use `hyperfine` against
//! a release build, e.g.:
//!
//! ```sh
//! cargo build --release
//! hyperfine --warmup 5 './target/release/bs ls'
//! ```
//!
//! The benchmark also runs `list_worktrees_status` (the pre-change, expensive
//! per-slot-checking code path `bs list` used to call) at the same pool
//! sizes, purely as a comparison baseline showing the magnitude of the
//! improvement this proposal makes; it is not itself subject to the SLO.

use std::path::{Path, PathBuf};
use std::process::Command;

use criterion::{Criterion, criterion_group, criterion_main};
use tempfile::TempDir;

use bonsai::worktree;

/// A throwaway git repository with `n` real worktree slots created under a
/// `pool` subdirectory, suitable for benchmarking pool-scan functions that
/// shell out to `git worktree list --porcelain`.
struct BenchRepo {
    _root: TempDir,
    repo_dir: PathBuf,
    pool_dir: PathBuf,
}

fn run_git(dir: &Path, args: &[&str]) {
    assert_under_tempdir(dir, &format!("git {args:?}"));

    // Clear git-hook-injected environment variables (GIT_DIR, GIT_WORK_TREE,
    // etc.) the same way `bonsai::worktree`'s internal `git_cmd()` helper does.
    // Without this, running the benchmark from inside a git hook (e.g. via
    // lefthook's `pre-commit`) makes these `git` invocations operate on the
    // *hook's* repository instead of the throwaway one created here.
    let mut cmd = Command::new("git");
    for var in [
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_WORK_TREE",
        "GIT_PREFIX",
        "GIT_INTERNAL_SUPER_PREFIX",
        "GIT_COMMON_DIR",
    ] {
        cmd.env_remove(var);
    }
    // Belt-and-suspenders: explicitly pin the working directory via `-C`
    // rather than relying solely on `Command::current_dir` + cleared env, so
    // this is doubly protected against any ambient `GIT_*` environment state.
    let status = cmd
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("failed to spawn git");
    assert!(status.success(), "git {args:?} failed in {dir:?}");
}

/// Safety guard shared by every place this benchmark changes the process's
/// cwd or runs `git` against a directory: refuse to operate on anything that
/// isn't a throwaway path under the OS temp directory. This exists
/// specifically to prevent a repeat of an earlier incident where an ambient
/// `GIT_DIR`/`GIT_WORK_TREE` ended up pointed at the real project checkout
/// during manual testing, causing `git commit` to run against the real repo.
fn assert_under_tempdir(dir: &Path, action: &str) {
    let canonical_dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let canonical_tmp = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    assert!(
        canonical_dir.starts_with(&canonical_tmp),
        "refusing to {action} in {dir:?}: not under the OS temp directory \
         ({canonical_tmp:?}); this benchmark must only ever touch throwaway \
         temp repos"
    );
}

impl BenchRepo {
    fn new(n: usize) -> Self {
        let root = TempDir::new().expect("create tempdir");
        let repo_dir = root.path().join("repo");
        let pool_dir = root.path().join("pool");
        std::fs::create_dir_all(&repo_dir).unwrap();
        std::fs::create_dir_all(&pool_dir).unwrap();

        run_git(&repo_dir, &["init", "-q"]);
        run_git(&repo_dir, &["config", "user.email", "bench@example.com"]);
        run_git(&repo_dir, &["config", "user.name", "bench"]);
        std::fs::write(repo_dir.join("README.md"), "bench\n").unwrap();
        run_git(&repo_dir, &["add", "."]);
        run_git(&repo_dir, &["commit", "-q", "-m", "init"]);

        for i in 0..n {
            let slot = pool_dir.join(format!("{i:08x}"));
            run_git(
                &repo_dir,
                &[
                    "worktree",
                    "add",
                    "--detach",
                    slot.to_str().unwrap(),
                    "HEAD",
                ],
            );
        }

        Self {
            _root: root,
            repo_dir,
            pool_dir,
        }
    }
}

/// Pool sizes covering the SLO's two reference points (5 and 50 slots) plus a
/// couple of intermediate/edge points to see the scaling curve.
const POOL_SIZES: &[usize] = &[1, 5, 10, 25, 50];

/// RAII guard that restores the process's previous working directory on
/// drop (including on panic/unwind), and asserts the target directory is a
/// throwaway temp path before changing into it (see `assert_under_tempdir`).
struct CwdGuard {
    prev: PathBuf,
}

impl CwdGuard {
    fn enter(dir: &Path) -> Self {
        assert_under_tempdir(dir, "set the process cwd to");
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        Self { prev }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prev);
    }
}

fn bench_list_pool_worktrees(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_pool_worktrees");
    for &n in POOL_SIZES {
        let repo = BenchRepo::new(n);
        // `list_pool_worktrees` shells out to `git worktree list --porcelain`
        // in the current process's cwd, so scope the cwd change to this
        // benchmark's measurements.
        let _cwd_guard = CwdGuard::enter(&repo.repo_dir);

        group.bench_function(format!("{n}_slots"), |b| {
            b.iter(|| worktree::list_pool_worktrees(&repo.pool_dir).unwrap());
        });
    }
    group.finish();
}

/// Baseline comparison only (not itself SLO-checked): the pre-change,
/// per-slot `lsof` + `git status --porcelain` code path `bs list` used to
/// call before this proposal.
fn bench_list_worktrees_status_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_worktrees_status_baseline");
    for &n in POOL_SIZES {
        let repo = BenchRepo::new(n);
        let _cwd_guard = CwdGuard::enter(&repo.repo_dir);

        group.bench_function(format!("{n}_slots"), |b| {
            b.iter(|| worktree::list_worktrees_status(&repo.pool_dir).unwrap());
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_list_pool_worktrees,
    bench_list_worktrees_status_baseline
);
criterion_main!(benches);
