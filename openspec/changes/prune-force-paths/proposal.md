## Why

`bs prune` only deletes slots classified `available`, so a slot that is `in use`
(dirty, or holding an unmerged branch) can never be reclaimed through bonsai.
Users need a deliberate way to force such slots back into a reclaimable state,
without weakening the guarantee that `locked` slots are never touched.

## What Changes

- **Definition of pruning (made explicit in the spec):** pruning is cleaning
  up, not merely deleting. It brings eligible worktrees to `available`, then
  eliminates the extra available ones so exactly one remains (never zero, never
  many) to reclaim disk space.
- `bs prune` accepts zero or more optional worktree path arguments that narrow
  which slots are candidates for pruning. With no paths, every pool slot is a
  candidate (today's behavior).
- `bs prune` accepts a `--force` option, usable with or without paths.
- **`--force` semantics:** `in use` candidate slots become eligible for pruning.
  A forced slot is either deleted like any available slot, or — if it is the
  slot chosen to be preserved — _sanitized_ so it is genuinely `available`:
  - HEAD is reset to the repository's current HEAD commit, detached (any
    checked-out branch is detached from the slot; the branch itself is NOT
    deleted),
  - all uncommitted changes (staged and unstaged) are discarded,
  - all untracked files and directories are removed.
  - Git-ignored files are left alone (they do not affect classification and may
    be `bonsai.copy` files).
    After this, the slot classifies as `available` under the same rules as
    `bs list`/`bs status`.
- **`locked` slots can NEVER be pruned or modified**, with or without `--force`.
  Explicitly targeting a locked slot is an error telling the user to unlock it
  first. Locked slots that are not explicitly targeted (e.g. `bs prune --force`
  with no paths) are simply skipped.
- Without `--force`, explicitly targeting an `in use` slot is an error
  requiring `--force`.
- The "always preserve exactly one available slot" rule ALWAYS applies,
  including with paths and with `--force`. Preserving means keeping the
  worktree registered and on disk; it may still be reset/detached/cleaned.
- Paths are validated up front; any invalid target aborts the run with a
  non-zero exit before anything is deleted or modified.
- Behavior of plain `bs prune` (no paths, no `--force`) is unchanged.

## Capabilities

### New Capabilities

### Modified Capabilities

- `worktree-prune`: adds path targets, the `--force` option (sanitizing /
  deleting `in use` slots), the hard refusal to touch `locked` slots, and
  clarifies how preservation interacts with them.

## Impact

- `apps/bonsai/src/program.ts`: `prune` arguments, option, help text, output.
- `packages/core/src/pool.ts` (+ `types.ts`, `ports.ts`): extended prune use
  case; the git port needs a way to hard-reset and clean a slot.
- `packages/git/src/git.ts`: adapter implementation of that operation.
- Tests: core unit tests, `program.test.ts`, e2e `worktree_prune.test.ts`.
- `README.md` / `skills/bonsai/SKILL.md` mention prune; docs are NOT changed
  without explicit permission.
