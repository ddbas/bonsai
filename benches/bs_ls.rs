//! Criterion benchmark for `bs ls` (`Commands::List`)'s underlying per-slot
//! classification path, tracking the performance SLO defined in
//! `openspec/changes/add-bs-status-command/specs/worktree-list/spec.md`:
//!
//! `bs list` now classifies each slot via `classify_slot_status`, which
//! short-circuits: a locked slot never invokes `git status`/`lsof`; a dirty
//! unlocked slot never invokes `lsof`; only a clean, unlocked slot needs
//! both. This benchmark measures `list_worktrees_status` (the
//! `classify_slot_status`-based, thread-per-slot pool scan `bs list` calls)
//! across pool sizes (1/5/10/25/50 slots) under three slot-mix scenarios:
//!
//! 1. **All locked** — the fastest path (no `git status`/`lsof` calls).
//! 2. **All dirty, unlocked** — the middle path (`git status` only).
//! 3. **All clean, unlocked, available** — the worst case (`git status` +
//!    `lsof` for every slot); structurally identical in cost to the
//!    pre-change per-slot fan-out this proposal's early return does not
//!    eliminate for this scenario.
//!
//! A comparison-only baseline benchmark (`baseline_no_short_circuit`) also
//! runs `is_clean` and `has_open_files` unconditionally for every slot
//! (ignoring the early-return short circuit), reproducing the shape of the
//! pre-change `list_worktrees_status` cost profile for reference; it is not
//! itself subject to any SLO.
//!
//! This benchmarks the in-process library calls that back `bs list`, not the
//! full `bs` process spawn, so it isolates the cost this proposal targets
//! from unrelated process-startup overhead. For whole-process latency
//! (including binary startup), use `hyperfine` against a release build, e.g.:
//!
//! ```sh
//! cargo build --release
//! hyperfine --warmup 5 './target/release/bs ls'
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use criterion::{Criterion, criterion_group, criterion_main};
use tempfile::TempDir;

use bonsai::worktree;

/// A throwaway git repository with `n` real worktree slots created under a
/// `pool` subdirectory, suitable for benchmarking pool-scan functions that
/// shell out to `git worktree list --porcelain` / `git status` / `lsof`.
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

/// Slot-mix scenario for a [`BenchRepo`].
#[derive(Clone, Copy)]
#[allow(clippy::enum_variant_names)]
enum Scenario {
    /// Every slot is git-locked (via `git worktree lock`).
    AllLocked,
    /// Every slot is unlocked but has an untracked file (dirty).
    AllDirty,
    /// Every slot is unlocked and clean (the "available" case).
    AllAvailable,
}

impl BenchRepo {
    fn new(n: usize, scenario: Scenario) -> Self {
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
            match scenario {
                Scenario::AllLocked => {
                    run_git(&repo_dir, &["worktree", "lock", slot.to_str().unwrap()]);
                }
                Scenario::AllDirty => {
                    std::fs::write(slot.join("dirty.txt"), "dirty").unwrap();
                }
                Scenario::AllAvailable => {}
            }
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

/// Benchmark `list_worktrees_status` (the `classify_slot_status`-based path
/// `bs list` calls) under a given scenario, across all `POOL_SIZES`.
fn bench_scenario(c: &mut Criterion, group_name: &str, scenario: Scenario) {
    let mut group = c.benchmark_group(group_name);
    group.sample_size(20);
    group.measurement_time(std::time::Duration::from_secs(2));
    group.warm_up_time(std::time::Duration::from_millis(500));
    for &n in POOL_SIZES {
        let repo = BenchRepo::new(n, scenario);
        let _cwd_guard = CwdGuard::enter(&repo.repo_dir);

        group.bench_function(format!("{n}_slots"), |b| {
            b.iter(|| worktree::list_worktrees_status(&repo.pool_dir).unwrap());
        });
    }
    group.finish();
}

fn bench_classify_slot_status_all_locked(c: &mut Criterion) {
    bench_scenario(c, "classify_slot_status_all_locked", Scenario::AllLocked);
}

fn bench_classify_slot_status_all_dirty(c: &mut Criterion) {
    bench_scenario(c, "classify_slot_status_all_dirty", Scenario::AllDirty);
}

fn bench_classify_slot_status_all_available(c: &mut Criterion) {
    bench_scenario(
        c,
        "classify_slot_status_all_available",
        Scenario::AllAvailable,
    );
}

/// Comparison-only baseline (not itself SLO-checked): runs both `is_clean`
/// and `has_open_files` unconditionally for every slot, ignoring the
/// early-return short circuit `classify_slot_status` uses. Reproduces the
/// cost shape of the pre-change `list_worktrees_status` implementation
/// (which always ran both `git status` and `lsof` per slot) for reference.
fn bench_baseline_no_short_circuit(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_no_short_circuit");
    group.sample_size(20);
    group.measurement_time(std::time::Duration::from_secs(2));
    group.warm_up_time(std::time::Duration::from_millis(500));
    for &n in POOL_SIZES {
        let repo = BenchRepo::new(n, Scenario::AllAvailable);
        let _cwd_guard = CwdGuard::enter(&repo.repo_dir);

        group.bench_function(format!("{n}_slots"), |b| {
            b.iter(|| {
                let entries = worktree::list_pool_worktrees(&repo.pool_dir).unwrap();
                let handles: Vec<_> = entries
                    .into_iter()
                    .map(|entry| {
                        std::thread::spawn(move || {
                            let _dirty = !worktree::is_clean(&entry.path).unwrap();
                            let _has_processes = worktree::has_open_files(&entry.path).unwrap();
                        })
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_classify_slot_status_all_locked,
    bench_classify_slot_status_all_dirty,
    bench_classify_slot_status_all_available,
    bench_baseline_no_short_circuit
);
criterion_main!(benches);
