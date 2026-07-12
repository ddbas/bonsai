## 1. Worktree Status Model

- [x] 1.1 Add `Locked` variant to `WorktreeStatus` enum in `src/worktree/mod.rs`
- [x] 1.2 Update `list_worktrees_status`: when a slot's `locked` flag is true,
      return `WorktreeStatus::Locked` (still collect and return `WorktreeStats`
      for display); `locked` takes priority over `InUse` signals
- [x] 1.3 Update unit tests for `list_worktrees_status` classification: add
      cases for locked+clean, locked+dirty, locked+open-processes — all must
      yield `WorktreeStatus::Locked`

## 2. Worktree Library Functions

- [x] 2.1 Add `lock_worktree(path: &Path, reason: Option<&str>) -> Result<()>`
      to `src/worktree/mod.rs` — shells out to
      `git worktree lock [--reason <msg>] <path>`
- [x] 2.2 Add `unlock_worktree(path: &Path) -> Result<()>` to
      `src/worktree/mod.rs` — shells out to `git worktree unlock <path>`
- [x] 2.3 Add `validate_pool_slot(path: &Path, pool_dir: &Path) -> Result<()>` —
      verifies the path falls under the pool and exists; returns a clear error
      if not
- [x] 2.4 Write unit tests for `validate_pool_slot`: valid pool path, path
      outside pool, non-existent path

## 3. CLI Subcommands

- [x] 3.1 Add `Lock` variant to the `Commands` enum in `src/main.rs` with
      optional `--reason <string>` flag and optional positional `[<path>]`
      argument
- [x] 3.2 Add `Unlock` variant to the `Commands` enum in `src/main.rs` with
      optional positional `[<path>]` argument
- [x] 3.3 Implement the `Lock` dispatch arm: resolve pool dir, resolve target
      path (argument or `current_worktree()`), call `validate_pool_slot`, call
      `lock_worktree`, print a confirmation line on success
- [x] 3.4 Implement the `Unlock` dispatch arm: resolve pool dir, resolve target
      path (argument or `current_worktree()`), call `validate_pool_slot`, call
      `unlock_worktree`, print a confirmation line on success

## 4. `bs list` Display Update

- [x] 4.1 Add the yellow `locked` badge rendering branch to the `bs list`
      dispatch arm for `WorktreeStatus::Locked` rows (using `owo-colors`
      `.yellow()`)
- [x] 4.2 Ensure stats icons (`⚙N`, `±N`, `?N`) are rendered on `Locked` rows
      the same way they are on `InUse` rows

## 5. TODO Cleanup

- [x] 5.1 Remove the `lock & unlock subcommand` TODO item from `TODO.md`

## 6. Integration Tests

- [x] 6.1 Add an integration test for `bs lock` with an explicit path argument —
      creates a real pool slot, locks it by path, verifies `list_pool_worktrees`
      sees it as locked
- [x] 6.2 Add an integration test for `bs unlock` — locks a slot then unlocks
      it, verifies `WorktreeStatus` returns to `Available`
- [x] 6.3 Add an integration test for `bs lock --reason` — verifies the reason
      is stored (git porcelain output includes `locked <reason>`)
- [x] 6.4 Add an integration test for the default-to-current-slot behaviour of
      `bs lock` / `bs unlock` (no path argument, run from inside a slot)
- [x] 6.5 Add an integration test verifying `bs list` output shows the yellow
      `locked` badge for a locked slot (and that a locked+dirty slot is shown as
      `locked`, not `in use`)
