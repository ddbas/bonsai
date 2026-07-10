## Why

`bs` runs `lsof +D <slot>` — a **recursive filesystem traversal** — on every
pool slot, every time the user invokes `bs get` or `bs list`. On a Node.js repo
with `node_modules` (~40 k files), each `lsof +D` call takes 1.1–1.3 s; with
three managed slots that is **3–4 s of blocking lsof overhead before a single
git command fires**. On larger repos (200 k+ files) this easily exceeds 10 s.
Beyond lsof, two separate git subprocesses are spawned to obtain information
that can be fetched in a single call, and `git worktree prune` runs
unconditionally on every `bs get` regardless of whether any stale entries exist.

## What Changes

- **BREAKING** `has_open_files` / `count_open_processes` switch from `lsof +D`
  (recursive scan, O(files-in-tree)) to `lsof +d` (non-recursive, O(1) stat of
  top-level directory only). A process whose CWD is the slot root is still
  detected; an editor with a file open deep in the tree is not — but the
  `git status --porcelain` check already handles the case where that editor has
  created dirty files.
- `git rev-parse HEAD` and `git rev-parse --git-common-dir` are merged into a
  single subprocess call (`git rev-parse HEAD --git-common-dir`), eliminating
  one process-spawn round-trip per invocation.
- `git worktree prune` is moved off the unconditional hot path: it is now
  triggered only when `git worktree list --porcelain` surfaces at least one
  registered slot whose directory no longer exists on disk.
- Per-slot availability checks in `list_worktrees_status` (used by both `bs get`
  and `bs list`) are parallelised using `std::thread::spawn`, so wall-clock time
  scales with the slowest slot rather than the sum of all slots.

## Capabilities

### New Capabilities

_(none — all changes are to existing capabilities)_

### Modified Capabilities

- `worktree-open-file-detection`: replace `lsof +D` (recursive) with `lsof +d`
  (non-recursive); update availability semantics accordingly.
- `worktree-get`: merge git rev-parse subprocesses; make `git worktree prune`
  conditional on detected stale entries; update availability definition to match
  the new `lsof +d` semantics.
- `worktree-list`: update availability definition to match the new `lsof +d`
  semantics; parallelise per-slot stat collection.

## Impact

- `src/worktree/mod.rs`: all four changes land here — `run_lsof` /
  `run_lsof_count` (flag change), `get_worktree` (merged rev-parse, lazy prune),
  `list_worktrees_status` (parallelism via threads).
- Existing unit tests for `has_open_files` and `count_open_processes` remain
  valid; tests that verify `lsof +D` flag strings need updating.
- No new dependencies; thread parallelism uses `std::thread` from the standard
  library.
- The `worktree-open-file-detection` spec, `worktree-get` spec, and
  `worktree-list` spec each need delta updates to reflect revised availability
  semantics and the merged git call contract.
