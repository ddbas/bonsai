## 1. Branch state module

- [ ] 1.1 Add `src/worktree/branch_state.rs` with `resolve_default_ref()`
      (origin/HEAD → origin/main|master → local main|master only without remote)
      and verify unit/E2E tests for each fallback and the no-default-ref case
      pass
- [ ] 1.2 Implement `branch_state(slot, branch, &DefaultRef) -> BranchState`
      with ancestry check and `is_available()`, and verify E2E tests for fresh
      branch, detached-at-merged, ff merge and merge-commit merge are
      `available`
- [ ] 1.3 Add pushed-proof (`branch.<b>.remote/.merge`), remote-gone
      (`refs/remotes/origin/<merge-branch>`), `git cherry` and
      `git merge-tree --write-tree` checks, and verify E2E tests for rebase
      merge, squash merge (D moved on), remote present, remote stale, never
      pushed, abandoned and detached-with-commits
- [ ] 1.4 Produce reason text (commits ahead via `rev-list --count`, no
      upstream, remote exists, default unknown) and verify unit tests on the
      reason strings

## 2. Classification wiring

- [ ] 2.1 Change `classify` to `classify(locked, dirty, has_unmerged_work)` and
      verify its pure unit tests pass without git
- [ ] 2.2 Add `--untracked-files=all` to the dirty check and verify a test with
      a file inside a new untracked directory is `in use`
- [ ] 2.3 Wire `find_available_slot` (bs get) and `list_worktrees_status`
      (list/prune) to resolve D once and use branch state with early return;
      verify the `classification_agreement` test (list vs status) passes
- [ ] 2.4 Replace the processes section of `bs status` with the "Branch state"
      line and `-u` hint; verify status E2E tests for the new output

## 3. Remove lsof

- [ ] 3.1 Delete `run_lsof`, `has_open_files`, `list_open_processes`,
      `parse_lsof_processes`, `ProcessHandle`, `SlotStatusReport.processes` and
      verify `cargo build` has no references left (`rg lsof src`)
- [ ] 3.2 Remove lsof unit tests, lsof helpers in `tests/common`,
      `worktree_get*.rs`, `worktree_list.rs`, and the process-based tests; add a
      regression test that a clean merged slot with a process in it is
      `available`; verify `cargo test` passes
- [ ] 3.3 Update `--help` text in `src/main.rs`, `benches/bs_ls.rs`,
      `scripts/check-bs-ls-perf.sh`; verify `rg -i lsof` finds nothing outside
      archived specs

## 4. Docs and perf

- [ ] 4.1 Update README: new availability semantics, reuse of clean-but-active
      slots (use `bs lock`), stale refs until `git fetch --prune`, `push -u` /
      `push.autoSetupRemote` tip; verify by review
- [ ] 4.2 Rerun the `bs_ls` bench (`mise run bench`) and verify
      `scripts/check-bs-ls-perf.sh` passes the SLOs
