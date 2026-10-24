## Why

Slot availability is currently decided by `lsof`: a clean, unlocked slot is
`in use` if any process has a file open at its root. That is slow (one `lsof`
per slot), platform-fragile (hard error when `lsof` is missing), and answers the
wrong question. What matters for reuse is whether resetting the slot could lose
work, and git already knows that: a slot is safe to reuse when its branch has
nothing unmerged.

## What Changes

- **BREAKING**: Drop `lsof` and all open-process detection. Open processes no
  longer affect classification; `bs status` no longer lists processes; a missing
  `lsof` is no longer an error.
- Classification order becomes `locked` > dirty > unmerged branch work >
  `available`, computed by the same single shared function for `bs list`,
  `bs status`, `bs get`, and `bs prune`.
- A new "branch state" check decides whether a clean slot carries unmerged work,
  using only local git state (no network, no fetch):
  - HEAD is an ancestor of the default ref ⇒ nothing unmerged ⇒ available.
  - Otherwise, for squash/rebase merges: branch has upstream config, its
    remote-tracking ref is gone, and its content is already on the default ref
    (`git cherry` for rebase, `git merge-tree --write-tree` for squash) ⇒
    available.
  - Anything else, including an unresolvable default ref, detached HEAD with own
    commits, or git too old for `merge-tree --write-tree` ⇒ `in use`.
- Default ref is `origin/HEAD` → `origin/main|master` (local `main|master` only
  if there is no remote). The remote is always `origin`.
- Dirty check uses `--untracked-files=all`.
- `bs status` replaces the process section with a "Branch state" line that
  explains why a slot is in use (e.g. commits ahead of `origin/main`, no
  upstream configured).
- README documents that active-but-clean slots can be reused (use `bs lock`) and
  the `push -u` / `push.autoSetupRemote` tip.

## Capabilities

### New Capabilities

- `worktree-branch-state`: how a slot's branch is judged to carry unmerged work
  (default ref resolution, ancestry, rewritten-merge detection, reason text).

### Modified Capabilities

- `worktree-get`: availability scan uses branch state instead of `lsof`.
- `worktree-list`: badge classification and short-circuiting use branch state;
  performance SLO wording no longer references `lsof`.
- `worktree-status`: classification uses branch state; process listing and
  `lsof` hard error removed; branch-state reason added.
- `worktree-prune`: scenario wording only (in-use no longer means open-file).
- `worktree-open-file-detection`: all requirements removed.

## Impact

- Code: `src/worktree/mod.rs` (`classify`, status/list/get paths), new
  `src/worktree/branch_state.rs`; remove `lsof` helpers, `ProcessHandle`,
  `SlotStatusReport.processes`, `src/main.rs` help text.
- Tests: remove lsof-based tests/helpers in `tests/`; add E2E per outcome in the
  `GitEnv` harness with a bare `origin` repo.
- Bench/scripts: `benches/bs_ls.rs`, `scripts/check-bs-ls-perf.sh`.
- Docs: README.
- Behavior risk: a clean slot with an editor/agent running can now be reused.
