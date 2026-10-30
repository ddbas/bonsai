## Why

`bs prune` only deletes slots classified `available`, so a slot that is `in use`
(dirty or holding an unmerged branch) can never be reclaimed through bonsai.
Users need a deliberate, explicit way to force-remove specific slots, without
weakening the guarantee that `locked` slots are never destroyed.

## What Changes

- `bs prune` accepts zero or more optional worktree path arguments.
- `bs prune` accepts a `--force` option. With one or more paths, `--force`
  permits pruning slots classified `in use`.
- `locked` slots can NEVER be pruned, with or without `--force`. Targeting one
  is an error telling the user to unlock it first.
- When paths are given, only those slots are pruned (no other slots are
  touched, and the "preserve one available slot" rule does not apply).
- All targets are validated before anything is deleted; any invalid target
  (unmanaged path, locked, or in use without `--force`) aborts the run with a
  non-zero exit and no deletions.
- `--force` without paths is an error.
- Behavior of `bs prune` with no arguments is unchanged.

## Capabilities

### New Capabilities

### Modified Capabilities

- `worktree-prune`: adds targeted pruning by path, the `--force` option for
  `in use` slots, and the hard refusal to prune `locked` slots.

## Impact

- `apps/bonsai/src/program.ts`: `prune` command arguments/option and output.
- `packages/core/src/pool.ts` (+ `types.ts`): new targeted prune use case.
- Tests: core unit tests, `program.test.ts`, and e2e `worktree_prune.test.ts`.
- `README.md` / `skills/bonsai/SKILL.md` mention prune; docs are NOT changed
  without explicit permission.
