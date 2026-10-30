## 1. Git port and adapter

- [ ] 1.1 Add a port method to hard-reset and clean a slot (`reset --hard` +
      `clean -fd`, no `-x`) in `packages/core/src/ports.ts`, fakes in
      `packages/core/tests/fakes.ts`
- [ ] 1.2 Implement it in `packages/git/src/git.ts`

## 2. Core

- [ ] 2.1 Extend `prunePool` with `{ paths, force }`: candidate selection,
      up-front validation (unmanaged, locked, in use without force), aggregated
      error
- [ ] 2.2 Preserved-slot selection order (untargeted available > available
      candidate > forced in-use candidate); never choose locked or stale slots
- [ ] 2.3 Sanitize a forced in-use preserved slot (reset hard, clean, detach at
      repo HEAD); failure handling as with detach failures
- [ ] 2.4 Delete remaining eligible candidates; single `git worktree prune`
- [ ] 2.5 Unit tests in `packages/core/tests/pool.test.ts` covering every spec
      scenario

## 3. CLI

- [ ] 3.1 Add `[paths...]` and `--force` to `prune` in
      `apps/bonsai/src/program.ts`; update help text
- [ ] 3.2 Tests in `apps/bonsai/src/program.test.ts`

## 4. E2E

- [ ] 4.1 Cases in `apps/bonsai/tests/e2e/worktree_prune.test.ts`: force
      with/without paths, locked refused (with/without force, untouched when
      implicit), sanitized preserved slot reports `available`, branch survives,
      unlock then prune

## 5. Verify

- [ ] 5.1 `mise run ci` passes; commit and push
