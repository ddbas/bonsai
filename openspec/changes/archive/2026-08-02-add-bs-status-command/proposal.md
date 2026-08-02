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

- Remove only the usage-stats column (`⚙N ±N ?N`) from `bs list` / `bs ls`
  output. The status badge (`available` / `in use` / `locked`) is **retained**.
- `bs list` still checks lock state, dirty/untracked files, and open processes
  per slot to compute the badge, but does so with early-return short-circuiting:
  a locked slot is classified without touching git/lsof at all (lock state comes
  free from `git worktree list --porcelain`); a dirty slot is classified
  `in use` from `git status --porcelain` alone, without ever invoking `lsof`.
  `lsof` is only invoked for slots that are otherwise clean and unlocked, to
  distinguish `in use` (open processes) from `available`. This keeps the common
  cases (locked or dirty pools) fast while an all-clean pool still pays the same
  per-slot `lsof` cost the old implementation did — that cost is inherent to
  answering "is this slot open by a process" and is not reducible without
  changing what the badge reports.
- **BREAKING**: `bs list` / `bs ls` output format changes — the trailing stats
  column is removed from every row. Scripts that parse `bs list` output for this
  column will need to switch to `bs status`. The status badge column
  format/values are unchanged from before this proposal.
- The status-badge classification logic (`locked` > `in use` > `available`) is
  factored into one shared function used by both `bs list` (fed by
  early-returning boolean checks) and `bs status` (fed by the full itemized
  detail it collects anyway), so the two commands can never disagree on how a
  slot is classified.
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

- `worktree-list`: `bs list` / `bs ls` no longer prints a usage-stats column and
  no longer performs full per-slot `lsof`/`git status` fan-out for stats
  purposes; it still computes and prints the status badge, using early-return
  boolean checks (`is_clean`/`has_open_files`) that skip `lsof` for locked or
  dirty slots. A performance SLO is now a documented requirement, revised to
  reflect that an all-available pool still requires one `lsof` call per slot
  (see Performance Benchmarking in design.md); enforced by an automated
  benchmark.
- `worktree-usage-stats`: the usage-stats rendering (open-process count,
  uncommitted count, untracked count, and now the underlying detail behind each
  count) moves from `bs list`'s compact column to `bs status`'s detailed report.
  The status badge itself remains in `bs list`.

## Impact

- `src/main.rs`: remove only the stats-column rendering from the `List` command
  handler, keeping the status-badge rendering; add a `Status` subcommand and its
  handler.
- `src/worktree/mod.rs`: introduce a single shared `classify` function encoding
  the `locked` > `in use` > `available` priority rule, called both by a new
  early-return `classify_slot_status` helper (used by `bs list`'s per-slot
  threads, checking `locked` → `is_clean` → `has_open_files` in that order,
  short-circuiting as soon as the classification is known) and by `slot_status`
  (used by `bs status`, deriving its booleans from the full detail it already
  collects via `git_status_lines`/`list_open_processes`). `WorktreeStats` and
  the count-only helpers (`count_git_status_files`, `count_open_processes`) are
  removed as dead code once `find_available_slot` switches to the
  already-existing boolean sibling `has_open_files` (same semantics, one fewer
  redundant implementation). `list_worktrees_status` keeps its existing
  thread-per-slot concurrency, now calling `classify_slot_status` instead of
  computing stats.
- Tests: `tests/worktree_list.rs` scenarios asserting the stats column is gone
  need updating, while badge assertions are retained; add coverage asserting
  `lsof` is skipped for locked/dirty slots (the early-return behavior). New
  `tests/worktree_status.rs` (or similar) for the new subcommand.
- Performance: `benches/bs_ls.rs` (Criterion) benchmarks `bs list`'s per-slot
  classification cost across pool sizes and across dirty/locked/available slot
  mixes; `scripts/check-bs-ls-perf.sh` enforces the revised SLOs documented in
  `specs/worktree-list/spec.md`, run via `mise run bench`, the `pre-commit`
  lefthook hook, and a dedicated CI job.
- Docs/help text: update `bs list`/`bs ls` and add `bs status` descriptions in
  the CLI's `--help` output.
