## 1. Worktree Library Functions

- [ ] 1.1 Add
      `resolve_slot_path(input: &str, pool_dir: &Path) -> Result<PathBuf>` to
      `src/worktree/mod.rs` — accepts an 8-char hex slug or absolute/tilde path,
      validates it falls under the pool, and returns the canonicalized `PathBuf`
- [ ] 1.2 Add `lock_worktree(path: &Path, reason: Option<&str>) -> Result<()>`
      to `src/worktree/mod.rs` — shells out to
      `git worktree lock [--reason <msg>] <path>`
- [ ] 1.3 Add `unlock_worktree(path: &Path) -> Result<()>` to
      `src/worktree/mod.rs` — shells out to `git worktree unlock <path>`
- [ ] 1.4 Write unit tests for `resolve_slot_path`: valid slug, valid absolute
      path, unknown slug, non-pool path, and no-argument fallback (mock pool dir
      with a temp directory)

## 2. CLI Subcommands

- [ ] 2.1 Add `Lock` variant to the `Commands` enum in `src/main.rs` with
      optional `--reason <string>` flag and optional positional `[<worktree>]`
      argument
- [ ] 2.2 Add `Unlock` variant to the `Commands` enum in `src/main.rs` with
      optional positional `[<worktree>]` argument
- [ ] 2.3 Implement the `Lock` dispatch arm: resolve pool dir and slot path
      (defaulting to current slot), call `lock_worktree`, print confirmation on
      success
- [ ] 2.4 Implement the `Unlock` dispatch arm: resolve pool dir and slot path
      (defaulting to current slot), call `unlock_worktree`, print confirmation
      on success

## 3. TODO Cleanup

- [ ] 3.1 Remove the `lock & unlock subcommand` TODO item from `TODO.md`

## 4. Integration Tests

- [ ] 4.1 Add an integration test (in `tests/`) for `bs lock` with a slug
      argument — creates a real pool slot, locks it by slug, verifies
      `list_pool_worktrees` sees it as locked
- [ ] 4.2 Add an integration test for `bs unlock` — locks a slot then unlocks
      it, verifies it returns to available status
- [ ] 4.3 Add an integration test for `bs lock --reason` — verifies the reason
      is stored (git porcelain output includes `locked <reason>`)
- [ ] 4.4 Add an integration test for `bs lock` / `bs unlock` with no argument
      while inside a slot (default-to-current-slot behaviour)
