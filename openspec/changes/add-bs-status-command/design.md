## Context

`bs list` currently calls `worktree::list_worktrees_status`, which for every
pool slot spawns a thread that runs `lsof -w +d <slot>` and
`git status --porcelain -C <slot>` concurrently, then classifies the slot as
`Available` / `InUse` / `Locked`. This is bounded by the _slowest single slot_
(good), but the per-slot cost is still real — `lsof` scans the whole open-file
table and `git status` walks the working tree — and on a pool with dozens of
worktrees the aggregate wall-clock and system load add up even with concurrency.
`bs list`'s job (enumerate what exists) does not need this data; only a "should
I touch/reuse this slot right now?" question does.

`bs get`'s `find_available_slot` also depends on the same lock/dirty/open-file
checks to decide whether a slot can be reused — that usage is unaffected by this
change; it does not go through `bs list`'s code path and keeps running its own
per-slot checks (it only ever needs to check slots until it finds one free, not
enumerate every slot).

## Goals / Non-Goals

**Goals:**

- Make `bs list` / `bs ls` O(cheap): only `git worktree list --porcelain`
  parsing, no `lsof`, no `git status`, regardless of pool size.
- Introduce `bs status [PATH]` as the single place to get lock / dirty /
  open-process detail for one slot, resolved either from an explicit path or
  from the current working directory.
- Preserve the exact classification semantics (`locked` beats `in use` beats
  `available`) that `bs list` used to apply, just relocated to `bs status`.
- Give more actionable detail than the old compact stats column: which files are
  dirty/untracked (from `git status --porcelain` lines) and which processes
  (PID + command) hold the slot open, not just counts.

**Non-Goals:**

- Changing `bs get`'s slot-selection logic (`find_available_slot`) or its
  performance characteristics — out of scope.
- Adding a `--all`/bulk mode to `bs status` that re-introduces per-slot scanning
  across the whole pool (would reintroduce the exact problem this change fixes).
  If bulk detail is wanted later, it's a separate proposal.
- Machine-readable (JSON) output for `bs status` — text output only for now,
  matching the rest of the CLI's plain-text conventions.

## Decisions

### `bs list` stops calling `list_worktrees_status`

`Commands::List` switches from `worktree::list_worktrees_status(&pool_dir)` to
`worktree::list_pool_worktrees(&pool_dir)`, which only parses
`git worktree list --porcelain` (already fast, single git invocation, no
subprocess fan-out). Each row keeps: the `▶`/`(current)` current-slot marker,
the tilde-abbreviated path, and the optional bold branch suffix. The status
badge and stats column are removed entirely — including the free "locked" flag
already present in the porcelain output — for output consistency: a row's
presence/absence of a badge should not itself imply partial status information
is available; `bs status` is the single source of truth for slot state.

Alternative considered: keep showing the (free) `locked` badge in `bs list`
since it costs nothing beyond porcelain parsing, and only drop the
`lsof`/`git status`-derived parts. Rejected because it fragments "status" into
"the free part of status lives in `list`, the rest lives in `status`", which is
confusing; the proposal's ask is a clean split — `list` enumerates, `status`
inspects.

### `bs status` slot resolution

- No `PATH`: reuse `worktree::current_worktree()` (same function `bs current`
  uses) to find the slot containing the CWD. `None` → error: "not inside a
  managed bonsai pool slot; please provide a path argument" (mirrors the
  existing message used by `bs lock`/`bs unlock`).
- `PATH` given: validate via `worktree::validate_pool_slot(&path, &pool_dir)`
  (existing helper already used by `bs lock`/`bs unlock`), so the same
  existence + "must be inside this repo's pool" rules apply.

This reuses two already-tested code paths instead of inventing new resolution
logic.

### `bs status` detail computation

Introduce a small `SlotStatus` aggregate (name tentative) built from existing
primitives, reusing rather than duplicating logic:

- Lock state: from `git worktree list --porcelain` (`WorktreeEntry.locked`).
  Extend porcelain parsing to also capture the optional lock reason string (the
  `locked` line may be followed by a reason, e.g. `locked build in progress`) so
  `bs status` can display _why_ a slot is locked, which `bs list`'s badge never
  showed.
- Dirty/untracked detail: run `git status --porcelain` once and keep the parsed
  lines (not just counts) — reuses the same command ` count_git_status_files`
  already runs, but the new path returns the raw classified lines instead of
  only totals.
- Open processes: run `lsof -w +d <slot>` once and keep `(pid, command)` pairs
  deduplicated by PID — reuses the same command `count_open_processes` already
  runs, but returns the pairs instead of only a count.

Both `count_git_status_files` and `count_open_processes`/`parse_lsof_pids`
currently discard the detail needed here. Rather than changing their return
types (which would ripple into `bs list`... except `bs list` no longer calls
them) or duplicating the git/lsof invocation, add sibling functions in
`worktree/mod.rs` that return the detailed rows, and have the count-only helpers
either stay as thin wrappers or be reused directly by `find_available_slot`
(unaffected). This avoids parsing `lsof`/`git status` output twice for the same
call in `bs status`.

Overall classification for the report uses the same priority order as before:
`Locked` > `InUse` (dirty or open handles) > `Available`.

### Output format

Plain-text, human-readable, similar spirit to `bs info`'s `key: value` style but
with itemized sub-lists for files/processes, e.g.:

```
🌳 ~/.bonsai/myrepo/a3f9c1b2  (feature-x)
status: in use

open processes:
  1234  node
  5678  vim

uncommitted changes (2):
  M  src/main.rs
  A  src/lib.rs

untracked files (1):
  ?? notes.txt
```

For a locked slot, a `locked: <reason>` (or `locked` with no reason) line
replaces/augments `status: locked`. For a fully clean/idle slot, the
processes/uncommitted/untracked sections are omitted (or shown as explicit
"none" — deferred to implementation, matching the "more details" spirit without
demanding sections that add no information).

## Risks / Trade-offs

- **Breaking change for scripts parsing `bs list` output** → Called out
  explicitly as **BREAKING** in the proposal; `bs status` is the documented
  replacement for anything that needs per-slot state.
- **Users who watched `bs list` to eyeball "which slots are busy" lose that
  at-a-glance view** → Mitigated by keeping `bs list` fast/lightweight (its new
  purpose) and making `bs status` fast enough to run per-slot on demand; a
  future `watch bs status <path>` or shell loop covers the old use case without
  paying the cost on every `bs list` invocation.
- **Duplicated git/lsof invocation logic between the existing count-only helpers
  and the new detail-returning helpers** → Mitigated by factoring the common
  subprocess-invocation + line-parsing core so only the "what do we keep" step
  differs (counts vs. rows), rather than duplicating the `Command::new` setup.
- **Lock reason parsing is new and untested against exotic git porcelain
  output** → Mitigated by keeping it strictly additive (falls back to no reason
  string) and covering it with unit tests against representative porcelain
  snippets.

## Migration Plan

No data migration; this is a CLI-only, single-binary change with no persisted
state. Rollout is a normal release: update `bs list` and add `bs status`, update
help text and specs, ship in the next version. Users pin to an older `bs`
version if they depend on the old `bs list` columns until they migrate scripts
to `bs status`.

## Performance Benchmarking

To guard the performance goal this change makes (`bs list` becoming cheap and
not scaling with pool size) against future regression, a Criterion benchmark
(`benches/bs_ls.rs`) measures `worktree::list_pool_worktrees` across pool sizes
(1/5/10/25/50 slots) using real, throwaway git worktrees, and also benchmarks
the pre-change `worktree::list_worktrees_status` path at the same sizes purely
as a before/after comparison baseline (not itself SLO-checked).

`scripts/check-bs-ls-perf.sh` runs the benchmark and computes p95 per-iteration
latency from Criterion's raw sample data (Criterion's `estimates.json` only
reports mean/median/slope, not percentiles), then asserts the two SLOs codified
as a requirement in `specs/worktree-list/spec.md`:

1. p95 @ 50 slots <= 50ms.
2. p95 @ 50 slots / p95 @ 5 slots <= 2.5x.

Measured locally: the new `list_pool_worktrees` path scores ~6-7ms p95 @ 5 slots
and ~12-13ms p95 @ 50 slots (ratio ~1.7-2.0x), comfortably inside both bounds.
The old `list_worktrees_status` path (kept only as a benchmark comparison, not
part of `bs list` after this change) scores ~79ms @ 5 slots and ~600ms @ 50
slots (ratio ~7.6x), which is the magnitude of regression this benchmark is
designed to catch if per-slot subprocess fan-out is ever reintroduced into
`bs list`.

The check runs via `mise run bench` (cached via mise's `sources`/`outputs`, so
it's skipped when `Cargo.toml`/`Cargo.lock`/`src/**/*.rs`/`benches/**/*.rs` are
unchanged), is wired into the `pre-commit` lefthook hook (same file globs as the
existing `test` job) and into CI as a dedicated `bench` job in
`.github/workflows/ci.yml`.

## Open Questions

- Exact final text layout for `bs status` (e.g. whether to always print "none"
  for empty sections vs. omitting them) — left to implementation, constrained by
  the scenarios in `specs/worktree-status/spec.md`.
- Whether `bs status` should also print the lock reason for `bs list` in some
  reduced form — decided no (see Decisions); revisit only if user feedback asks
  for it.
