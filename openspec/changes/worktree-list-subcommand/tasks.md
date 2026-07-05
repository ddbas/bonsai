## 1. Worktree module — new helpers

- [x] 1.1 Add `tilde_path(path: &Path) -> String` helper to
      `src/worktree/mod.rs` that replaces the home directory prefix with `~`
      (use `dirs::home_dir()`)
- [x] 1.2 Add `WorktreeStatus` enum (`Available`, `InUse`) to
      `src/worktree/mod.rs`
- [x] 1.3 Add
      `list_worktrees_status(pool_dir: &Path) -> Result<Vec<(PathBuf, WorktreeStatus)>>`
      to `src/worktree/mod.rs`, reusing `list_pool_worktrees` and `is_clean`

## 2. CLI — `list` subcommand

- [x] 2.1 Add `List` variant to the `Commands` enum in `src/main.rs` with
      `#[command(alias = "ls")]` and a doc comment
- [x] 2.2 Implement the `List` handler in `src/main.rs`: resolve
      `managed_root()` + `repo_slug()`, check if pool dir exists (print friendly
      "no worktrees" message if not), call `list_worktrees_status`, and print
      one coloured line per slot using `owo-colors`
- [x] 2.3 Use green for `available` and red for `in use` in the output (e.g.
      `"available".green()` / `"in use".red()` via `owo-colors::OwoColorize`)

## 3. Unit tests — worktree module helpers

- [x] 3.1 Unit test `tilde_path`: path under home dir → tilde prefix; path
      outside home dir → unchanged
- [x] 3.2 Unit test `list_worktrees_status` — mock or directly test the status
      classification logic (locked → `InUse`, dirty → `InUse`, clean+unlocked →
      `Available`)

## 4. Integration tests

- [x] 4.1 Add integration test (using `GitEnv` + container) that runs `bs list`
      with no pool and asserts the friendly empty-pool message is printed and
      exit code is 0
- [x] 4.2 Add integration test that creates one available slot then runs
      `bs list` and asserts the output contains `available` and the
      tilde-formatted path
- [x] 4.3 Add integration test that creates one slot with uncommitted changes
      then runs `bs list` and asserts the output contains `in use`

## 5. Quality gates

- [x] 5.1 Run `mise run build` — zero compile errors
- [x] 5.2 Run `mise run test` — all tests pass
