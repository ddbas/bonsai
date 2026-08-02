## 1. Worktree module: shared classification + detail-returning helpers

- [ ] 1.1 Extend `list_pool_worktrees_checking_stale` porcelain parsing to
      capture the optional lock reason string from the `locked` line (e.g.
      `locked build in progress`) and add a `lock_reason: Option<String>` field
      to `WorktreeEntry`.
- [ ] 1.2 Add a detail-returning git-status helper (e.g.
      `git_status_lines(slot_path: &Path) -> Result<GitStatusDetail>`) that runs
      `git status --porcelain` once and returns the individual uncommitted and
      untracked lines (path + XY code), sharing the same underlying subprocess
      invocation as the existing boolean `is_clean` (factor out a shared
      `run_git_status_porcelain` core so both can share it without duplicating
      the `Command` setup).
- [ ] 1.3 Add a detail-returning lsof helper (e.g.
      `list_open_processes(path: &Path) -> Result<Vec<ProcessHandle>>`) that
      runs `lsof -w +d <path>` once and returns deduplicated `(pid, command)`
      pairs, sharing the same underlying subprocess invocation as the existing
      boolean `has_open_files` (factor out a shared `run_lsof_raw` core).
- [ ] 1.4 Add one shared classification function,
      `classify(locked: bool, dirty: bool, has_processes: bool) ->     WorktreeStatus`,
      encoding the `Locked` > `InUse` > `Available` priority rule in exactly one
      place.
- [ ] 1.5 Add
      `classify_slot_status(entry: &WorktreeEntry) ->     Result<WorktreeStatus>`:
      early-return classification for `bs list` — returns `Locked` immediately
      from `entry.locked` (no subprocess calls); else calls `is_clean` and
      returns `InUse` immediately if dirty (no `lsof` call); else calls
      `has_open_files` to decide the rest; routes the booleans it computed
      through `classify()`.
- [ ] 1.6 Add a `slot_status(path: &Path) -> Result<SlotStatusReport>` (naming
      tentative) function for `bs status` that combines lock state + reason,
      full git-status detail (`git_status_lines`), and full open-process detail
      (`list_open_processes`) into one struct, derives its classification
      booleans from that detail, and calls the same `classify()` used by
      `classify_slot_status` so the two commands cannot disagree.
- [ ] 1.7 Switch `find_available_slot` from `count_open_processes(&path)? > 0`
      to `has_open_files(&path)?` (identical semantics), then delete
      `count_git_status_files` and `count_open_processes` as dead code (only
      ever used by the now-removed stats column and by `find_available_slot`).
- [ ] 1.8 Add unit tests for the new lock-reason parsing (with and without a
      reason, and the existing no-lock case), for the detail helpers using the
      same synthetic/parsing test patterns already used for `parse_lsof_pids`
      and the boolean helpers, and for `classify()`/`classify_slot_status`
      covering all three priority-order combinations plus the early-return
      short-circuit behavior (e.g. asserting `lsof` is not invoked for a locked
      or dirty slot, via a fake `PATH`/spy).

## 2. `bs list` drops the stats column, keeps the badge

- [ ] 2.1 Change the `Commands::List` handler in `src/main.rs` to call the new
      classification path (per-slot `classify_slot_status`, still one thread per
      slot, joined in original order) instead of the old
      `list_worktrees_status`'s stats-computing body.
- [ ] 2.2 Remove the stats-column rendering logic from the `List` handler (the
      `Row` struct's `stats_str` field, `format_stats`), while **keeping** the
      status-badge rendering (the `status` field, the per-status `match` arms
      and colors) unchanged from before this proposal.
- [ ] 2.3 Remove `format_stats` and its unit tests in `src/main.rs` once
      confirmed unused (`bs status`'s renderer uses its own itemized format, not
      the compact stats string).
- [ ] 2.4 Update the `List` subcommand's clap doc comment to describe the
      revised output: badge retained, stats column removed, per-slot checks
      short-circuited.

## 3. `bs status` subcommand

- [ ] 3.1 Add a `Status { path: Option<PathBuf> }` variant to the `Commands`
      enum in `src/main.rs` with doc comments describing the optional path
      argument and its default-to-current-slot behavior.
- [ ] 3.2 Implement the `Commands::Status` match arm: resolve the target slot
      (via `worktree::current_worktree()` when `path` is `None`, erroring out
      with an actionable message if not inside a managed slot; via
      `worktree::validate_pool_slot` when `path` is `Some`), call
      `worktree::slot_status`, and render the report.
- [ ] 3.3 Implement the text renderer for the report: path + branch header,
      lock/classification line (with lock reason when locked), itemized open
      processes (PID + command), itemized uncommitted files, itemized untracked
      files — omitting or explicitly noting empty sections.
- [ ] 3.4 Wire up error propagation so `lsof` unavailability surfaces as a hard,
      actionable error (reusing the existing `has_open_files` error message
      conventions now that `count_open_processes` is removed per task 1.7).

## 4. Tests

- [ ] 4.1 Update `tests/worktree_list.rs`: remove assertions on the stats column
      while **restoring/keeping** assertions on the status badge (available/in
      use/locked, unchanged format); add tests asserting the early-return
      short-circuit — `lsof` is not invoked for a locked slot or a dirty slot
      (e.g. via a fake `PATH`/process-spy), and is invoked only for slots that
      are unlocked and clean.
- [ ] 4.2 Add `tests/worktree_status.rs` covering: explicit path invocation,
      default-to-current-slot invocation, error when CWD is not in a managed
      slot and no path given, error for a path outside the pool, classification
      priority (locked > in use > available) matching `bs list`'s badge for the
      same slot, lock reason display (with and without a reason), itemized
      process list, and itemized uncommitted/untracked file lists.
- [ ] 4.3 Add a unit test asserting `bs list`'s `classify_slot_status` and
      `bs status`'s `slot_status` agree on classification for the same slot
      across representative fixtures (locked, dirty, open-process, clean),
      guarding against the two call sites drifting apart now that they share
      `classify()` but gather signals differently.
- [ ] 4.4 Run the full test suite and fix any regressions.

## 5. Performance benchmarking

- [ ] 5.1 Add a Criterion dev-dependency and `benches/bs_ls.rs` benchmarking the
      new `classify_slot_status`-based per-slot path (the `bs list` pool-scan
      path) across pool sizes (1/5/10/25/50 slots) under three slot-mix
      scenarios — all locked, all dirty/unlocked, all clean/unlocked/available —
      using real, throwaway git worktrees and processes, plus a comparison-only
      baseline benchmark of the pre-change `worktree::list_worktrees_status`
      path at the same sizes.
- [ ] 5.2 Add `scripts/check-bs-ls-perf.sh`, computing p95 per-iteration latency
      from Criterion's raw sample data and asserting the revised, per-scenario
      SLOs in `specs/worktree-list/spec.md`: a strict absolute + scaling bound
      for the locked/dirty scenarios, and a scaling-only regression bound for
      the all-available scenario (see design.md's Performance Benchmarking
      section), exiting non-zero on violation.
- [ ] 5.3 Add a `mise run bench` task (with `sources`/`outputs` so it's skipped
      when unrelated files change, matching the `build`/`test` task pattern),
      wire it into the `pre-commit` lefthook hook (same file globs as `test`),
      and add a dedicated `bench` job to `.github/workflows/ci.yml`.
- [ ] 5.4 Once `bs list` switches to `classify_slot_status` (tasks 2.x), re-run
      the benchmark under all three scenarios to confirm the measured p95 values
      meet the revised SLOs end-to-end (not just at the library-function level),
      and record the all-available scenario's numbers in design.md for future
      reference.

## 6. Docs & polish

- [ ] 6.1 Update `bs help` / `--help` long-form docs (clap doc comments) to
      describe `bs status` and the revised `bs list` behavior (badge retained,
      stats column removed, early-return short-circuiting).
- [ ] 6.2 Update `README.md` (and any other user-facing docs mentioning
      `bs list`'s stats column) to describe the new split: badge stays on
      `bs list`, itemized detail moves to `bs status`.
- [ ] 6.3 Run `cargo fmt`, `cargo clippy`, and the full test suite; fix any
      warnings introduced by the change.
