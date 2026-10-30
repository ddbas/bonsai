## 1. Core

- [ ] 1.1 Add `pruneSlots(deps, paths, { force })` in `packages/core/src/pool.ts`
      with up-front validation (unmanaged, locked, in use without force) and
      deletion + single `git worktree prune`
- [ ] 1.2 Export it from `@bonsai/core` and reuse `PruneOutcome`
- [ ] 1.3 Unit tests in `packages/core/tests/pool.test.ts` (available, in use
      with/without force, locked with/without force, unmanaged, duplicates,
      all-or-nothing, deletion failure)

## 2. CLI

- [ ] 2.1 Add `[paths...]` argument and `--force` option to `prune` in
      `apps/bonsai/src/program.ts`; error on `--force` without paths; update
      command help text
- [ ] 2.2 Route to `pruneSlots` when paths are given; keep `prunePool` otherwise
- [ ] 2.3 Add tests to `apps/bonsai/src/program.test.ts`

## 3. E2E

- [ ] 3.1 Add e2e cases to `apps/bonsai/tests/e2e/worktree_prune.test.ts`
      (force in-use, locked refused with and without force, unlock then prune)

## 4. Verify

- [ ] 4.1 `mise run ci` passes; commit and push
