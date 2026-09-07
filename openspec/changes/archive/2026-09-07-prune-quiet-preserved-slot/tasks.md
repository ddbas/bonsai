## 1. Update CLI reporting

- [x] 1.1 Remove the `kept ...` println block for `outcome.preserved` in
      `src/main.rs`
- [x] 1.2 Broaden the "nothing to prune" early condition to include: pruned
      empty AND failures empty AND preserve_failure is none (regardless of
      whether `preserved` is `Some`)
- [x] 1.3 Verify preserve_failure reporting/exit-code path is unaffected

## 2. Update tests

- [x] 2.1 Update/remove any test asserting on `kept ...` stdout output
- [x] 2.2 Add/adjust a test asserting that pruning a pool with exactly one
      available slot (with a branch checked out) prints the same "nothing to
      prune" message as an empty pool, with no mention of the slot or branch
- [x] 2.3 Add/adjust a test asserting that pruning a pool with several available
      slots (one preserved, others deleted) prints no line identifying the
      preserved slot
- [x] 2.4 Confirm preserve-failure test still asserts the error line and
      non-zero exit status

## 3. Spec sync

- [x] 3.1 Sync `openspec/specs/worktree-prune/spec.md` with this change's delta
      spec
- [x] 3.2 Archive this change
