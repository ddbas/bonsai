## Why

`bs ls` is slow on repositories with many worktrees because it computes a
per-slot availability status (git-lock check, `git status --porcelain`, and a
non-recursive `lsof +d` scan) for _every_ managed slot before it can print a
single line. This makes the common case — "just show me my worktrees" — pay the
cost of the expensive case ("is this specific slot safe to reuse or inspect
right now?"). Splitting these concerns lets `bs ls` stay fast for large pools
while still giving users a way to get rich, on-demand status detail for the one
slot they actually care about.

## What Changes

- Remove the status badge (`available` / `in use` / `locked`) and the usage
  stats column (`⚙N ±N ?N`) from `bs list` / `bs ls` output. `bs list` no longer
  runs `lsof` or `git status --porcelain` for any slot; it only reads
  `git worktree list --porcelain` (already required to enumerate paths and
  branches), so its cost no longer scales with per-slot process/dirty-file
  checks.
- **BREAKING**: `bs list` / `bs ls` output format changes — the leading status
  badge and trailing stats column are removed from every row. Scripts that parse
  `bs list` output for these columns will need to switch to `bs status`.
- Add a new `bs status [PATH]` subcommand that reports detailed status for a
  single managed worktree slot:
  - `PATH` is optional; when omitted, `bs status` uses the bonsai pool slot
    containing the current working directory (same resolution as `bs current`)
    and errors out if the CWD is not inside a managed slot.
  - When `PATH` is given, it must resolve to a bonsai-managed pool slot for the
    current repository (validated the same way as `bs lock` / `bs unlock`).
  - Reports the same three signals `bs list` used to check — lock state,
    uncommitted/untracked files, and open-process file handles — but with more
    detail than the old compact stats column: the lock reason (if any), the
    individual `git status --porcelain` lines, and the PID + command name of
    each process with an open handle at the slot root (not just counts).
  - Still derives an overall classification (`available` / `in use` / `locked`)
    using the exact same priority rules `bs list` used to apply.

## Capabilities

### New Capabilities

- `worktree-status`: the `bs status [PATH]` subcommand — resolving the target
  slot (explicit path or current slot), computing lock/dirty/open-process
  detail, and rendering the detailed report and overall classification.

### Modified Capabilities

- `worktree-list`: `bs list` / `bs ls` no longer prints a status badge or
  usage-stats column and no longer performs per-slot `lsof`/`git status` checks;
  each row is reduced to the current-slot marker, path, and optional branch. A
  performance SLO is now a documented requirement (p95 latency <= 50ms at 50
  pool slots; scaling ratio 50-slot/5-slot p95 <= 2.5x), enforced by an
  automated benchmark.
- `worktree-usage-stats`: the usage-stats rendering (open-process count,
  uncommitted count, untracked count, and now the underlying detail behind each
  count) moves from `bs list`'s compact column to `bs status`'s detailed report.

## Impact

- `src/main.rs`: remove status-badge/stats rendering from the `List` command
  handler; add a `Status` subcommand and its handler.
- `src/worktree/mod.rs`: add a slot-resolution + detailed-status helper (reuse
  existing `count_git_status_files`, `count_open_processes`/`has_open_files`,
  and lock detection); `bs list` switches to calling only the cheap
  `list_pool_worktrees` (no threads, no `lsof`/`git status` per slot).
  `list_worktrees_status` and `WorktreeStatus`/`WorktreeStats` are repurposed
  for/consumed by the new `bs status` code path only.
- Tests: `tests/worktree_list.rs` scenarios asserting badges/stats columns need
  updating; new `tests/worktree_status.rs` (or similar) for the new subcommand.
- Performance: `benches/bs_ls.rs` (Criterion) benchmarks `bs list`'s pool-scan
  cost across pool sizes; `scripts/check-bs-ls-perf.sh` enforces the SLOs
  documented in `specs/worktree-list/spec.md`, run via `mise run bench`, the
  `pre-commit` lefthook hook, and a dedicated CI job.
- Docs/help text: update `bs list`/`bs ls` and add `bs status` descriptions in
  the CLI's `--help` output.
