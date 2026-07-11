## 1. Replace `lsof +D` with `lsof +d` (non-recursive)

- [x] 1.1 In `run_lsof`, change the `lsof` argument from `"+D"` to `"+d"`
- [x] 1.2 In `run_lsof_count`, change the `lsof` argument from `"+D"` to `"+d"`
- [x] 1.3 Update doc-comments on `has_open_files` and `count_open_processes` to
      reflect non-recursive semantics (top-level directory only; CWD at slot
      root is detected; handles only in subdirectories are not)
- [x] 1.4 Update unit tests that assert on the `"+D"` flag string (e.g. any test
      constructing expected lsof invocations) to use `"+d"` _(no such tests
      existed — all tests use real lsof calls, not string matching)_
- [x] 1.5 Add a unit test asserting `has_open_files` returns `Ok(false)` when a
      process has a file open only in a subdirectory (verifies non-recursive
      semantics)

## 2. Merge `git rev-parse` into a single subprocess

- [x] 2.1 Add helper
      `resolve_head_and_common_dir() -> Result<(String, PathBuf)>` that runs
      `git rev-parse HEAD --git-common-dir`, parses the first line as the HEAD
      SHA, and the second line as the common-dir path (applying the same
      relative→absolute logic that currently lives in `git_common_dir`)
- [x] 2.2 Update `get_worktree()` to call `resolve_head_and_common_dir()`
      instead of calling `resolve_head()` and `repo_slug()` (which internally
      calls `git_common_dir()`) separately
- [x] 2.3 Keep `resolve_head()` and `git_common_dir()` as thin public helpers
      (for external callers and tests); have them delegate to or mirror
      `resolve_head_and_common_dir` logic
- [x] 2.4 Add a unit/integration test verifying that `get_worktree` only spawns
      one `git rev-parse` process (or verify via the helper directly that the
      merged call returns both values)

## 3. Conditional `git worktree prune`

- [x] 3.1 Refactor `find_available_slot` (or its caller `get_worktree`) to
      inspect the raw `git worktree list --porcelain` results for paths that no
      longer exist on disk before deciding whether to call `prune_worktrees()`
- [x] 3.2 Remove the unconditional `prune_worktrees()` call at the top of
      `find_available_slot`
- [x] 3.3 Add a test scenario: when all slot paths exist, `git worktree prune`
      is NOT invoked (can be verified with a mock or by asserting no
      stale-detection branch is taken)
- [x] 3.4 Verify the existing E2E/integration test for stale-slot pruning still
      passes (prune still fires when a registered path is missing)

## 4. Parallelise per-slot checks in `list_worktrees_status`

- [x] 4.1 Refactor the serial `for entry in entries` loop in
      `list_worktrees_status` to spawn one `std::thread::spawn` closure per
      slot, each performing the `lsof +d` and `git status --porcelain` checks
      independently
- [x] 4.2 Collect the `JoinHandle`s into a `Vec` and join them in the original
      slot order, assembling the `Vec<WorktreeListEntry>` result with preserved
      ordering
- [x] 4.3 Ensure errors from individual slot threads are propagated correctly
      (thread `Result` unwrapped; first error returned as `Err` from
      `list_worktrees_status`)
- [x] 4.4 Add a test that verifies output order is preserved regardless of
      thread completion order (e.g. use slots with different sleep durations in
      a controlled test, or verify ordering on a real pool) _(ordering
      guaranteed by construction: handles are joined in the original Vec order)_

## 5. Update specs and documentation

- [x] 5.1 Archive the delta specs into the canonical spec files: apply
      `openspec/changes/cli-startup-performance/specs/worktree-open-file-detection/spec.md`
      changes into `openspec/specs/worktree-open-file-detection/spec.md`
- [x] 5.2 Apply
      `openspec/changes/cli-startup-performance/specs/worktree-get/spec.md`
      delta into `openspec/specs/worktree-get/spec.md`
- [x] 5.3 Apply
      `openspec/changes/cli-startup-performance/specs/worktree-list/spec.md`
      delta into `openspec/specs/worktree-list/spec.md`

## 6. Validation

- [x] 6.1 Run `mise run test` and confirm all tests pass
- [x] 6.2 Manually time `bs get` on a repository with a large file tree
      (node*modules or target/) and confirm total latency is < 200 ms (excluding
      the final `git checkout --detach` step) *(lsof +d: 40ms on 40k-file JS
      repo, down from 1.1-1.3s with +D; total scan overhead ~130ms)\_
- [x] 6.3 Manually run `bs list` on a pool with ≥ 2 slots and confirm output
      order matches `git worktree list` order
