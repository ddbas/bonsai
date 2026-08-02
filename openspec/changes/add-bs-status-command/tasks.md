## 1. Worktree module: detail-returning helpers

- [x] 1.1 Extend `list_pool_worktrees_checking_stale` porcelain parsing to
      capture the optional lock reason string from the `locked` line (e.g.
      `locked build in progress`) and add a `lock_reason: Option<String>` field
      to `WorktreeEntry`.
- [x] 1.2 Add a detail-returning git-status helper (e.g.
      `git_status_lines(slot_path: &Path) -> Result<GitStatusDetail>`) that runs
      `git status --porcelain` once and returns the individual uncommitted and
      untracked lines (path + XY code), reusing the same subprocess invocation
      `count_git_status_files` uses today (factor out the shared invocation so
      both can share it without duplicating the `Command` setup).
- [x] 1.3 Add a detail-returning lsof helper (e.g.
      `list_open_processes(path: &Path) -> Result<Vec<ProcessHandle>>`) that
      runs `lsof -w +d <path>` once and returns deduplicated `(pid, command)`
      pairs, reusing the shared invocation/parsing core `count_open_processes` /
      `parse_lsof_pids` use today.
- [x] 1.4 Add a `slot_status(path: &Path) -> Result<SlotStatusReport>` (naming
      tentative) function that combines lock state + reason, git-status detail,
      and open-process detail into one struct, and derives the overall
      `WorktreeStatus` classification using the existing priority rules
      (`Locked` > `InUse` > `Available`).
- [x] 1.5 Add unit tests for the new lock-reason parsing (with and without a
      reason, and the existing no-lock case) and for the detail helpers using
      the same synthetic/parsing test patterns already used for
      `parse_lsof_pids` and `count_git_status_files`.

## 2. `bs list` becomes lightweight

- [x] 2.1 Change the `Commands::List` handler in `src/main.rs` to call
      `worktree::list_pool_worktrees(&pool_dir)` instead of
      `worktree::list_worktrees_status(&pool_dir)`.
- [x] 2.2 Remove the status-badge and stats-column rendering logic from the
      `List` handler (the `Row` struct's `status`/`stats_str` fields, the
      per-status `match` arms, and `format_stats`), keeping only the
      `▶`/`(current)` marker, tilde path, and optional bold branch.
- [x] 2.3 Update/remove `format_stats` and its unit tests in `src/main.rs` if no
      longer used anywhere (confirm `bs status`'s renderer doesn't need the
      exact same compact format before deleting).
- [x] 2.4 Update the `List` subcommand's clap doc comment to drop the "coloured
      status badge" description and reflect the new lightweight output.

## 3. `bs status` subcommand

- [x] 3.1 Add a `Status { path: Option<PathBuf> }` variant to the `Commands`
      enum in `src/main.rs` with doc comments describing the optional path
      argument and its default-to-current-slot behavior.
- [x] 3.2 Implement the `Commands::Status` match arm: resolve the target slot
      (via `worktree::current_worktree()` when `path` is `None`, erroring out
      with an actionable message if not inside a managed slot; via
      `worktree::validate_pool_slot` when `path` is `Some`), call
      `worktree::slot_status`, and render the report.
- [x] 3.3 Implement the text renderer for the report: path + branch header,
      lock/classification line (with lock reason when locked), itemized open
      processes (PID + command), itemized uncommitted files, itemized untracked
      files — omitting or explicitly noting empty sections.
- [x] 3.4 Wire up error propagation so `lsof` unavailability surfaces as a hard,
      actionable error (reusing the existing `has_open_files`/
      `count_open_processes` error message conventions).

## 4. Tests

- [x] 4.1 Update `tests/worktree_list.rs` to remove/replace assertions on status
      badges and stats columns, asserting only path/branch/current markers
      remain, and add a test asserting `bs list` does not shell out to
      `lsof`/`git status` per slot (e.g. via a fake `PATH` or process-spy, or by
      asserting timing bounds mirrored on the existing performance-shaped tests
      if present).
- [x] 4.2 Add `tests/worktree_status.rs` covering: explicit path invocation,
      default-to-current-slot invocation, error when CWD is not in a managed
      slot and no path given, error for a path outside the pool, classification
      priority (locked > in use > available), lock reason display (with and
      without a reason), itemized process list, and itemized uncommitted/
      untracked file lists.
- [x] 4.3 Run the full test suite and fix any regressions.

## 5. Performance benchmarking

- [x] 5.1 Add a Criterion dev-dependency and `benches/bs_ls.rs` benchmarking
      `worktree::list_pool_worktrees` (the `bs list` pool-scan path) across pool
      sizes (1/5/10/25/50 slots) using real, throwaway git worktrees, plus a
      comparison-only baseline benchmark of the pre-change
      `worktree::list_worktrees_status` path at the same sizes.
- [x] 5.2 Add `scripts/check-bs-ls-perf.sh`, computing p95 per-iteration latency
      from Criterion's raw sample data and asserting the SLOs in
      `specs/worktree-list/spec.md` (p95 @ 50 slots <= 50ms; ratio p95@50/p95@5
      <= 2.5x), exiting non-zero on violation.
- [x] 5.3 Add a `mise run bench` task (with `sources`/`outputs` so it's skipped
      when unrelated files change, matching the `build`/`test` task pattern),
      wire it into the `pre-commit` lefthook hook (same file globs as `test`),
      and add a dedicated `bench` job to `.github/workflows/ci.yml`.
- [x] 5.4 Once `bs list` switches to `list_pool_worktrees` (tasks 2.x), re-run
      the benchmark to confirm the measured p95 values still meet both SLOs
      end-to-end (not just at the library-function level).

## 6. Docs & polish

- [x] 6.1 Update `bs help` / `--help` long-form docs (clap doc comments) to
      describe `bs status` and the revised `bs list` behavior.
- [x] 6.2 Update `README.md` (and any other user-facing docs mentioning
      `bs     list`'s status badge or stats column) to describe the new split
      between `bs list` and `bs status`.
- [x] 6.3 Run `cargo fmt`, `cargo clippy`, and the full test suite; fix any
      warnings introduced by the change.
