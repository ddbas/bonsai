## 1. Dependencies & Project Setup

- [ ] 1.1 Add `dirs = "5"` to `[dependencies]` in `Cargo.toml`
- [ ] 1.2 Add `uuid = { version = "1", features = ["v4"] }` to `[dependencies]`
      in `Cargo.toml`
- [ ] 1.3 Verify project builds cleanly after adding dependencies
      (`mise run build`)

## 2. Worktree Module — Core Utilities

- [ ] 2.1 Create `src/worktree/mod.rs` and declare the module in `src/lib.rs`
- [ ] 2.2 Implement `git_common_dir() -> Result<PathBuf>` — runs
      `git rev-parse --git-common-dir` in the current directory; errors on bare
      repos
- [ ] 2.3 Implement `repo_slug() -> Result<String>` — calls `git_common_dir()`,
      takes its parent as the main repo root, returns its basename lowercased
      with non-alphanumeric chars replaced by `-`
- [ ] 2.4 Implement `resolve_head() -> Result<String>` — runs
      `git rev-parse HEAD` in the current directory; returns the full commit SHA
- [ ] 2.5 Implement `managed_root() -> Result<PathBuf>` — returns
      `dirs::home_dir()/.bonsai`; errors with a clear message if home dir cannot
      be resolved
- [ ] 2.6 Implement `new_slot_path(pool_dir: &Path) -> PathBuf` — generates an
      8-char UUID v4 prefix and returns `pool_dir/<prefix>`

## 3. Worktree Module — Pool Scan

- [ ] 3.1 Implement
      `list_pool_worktrees(pool_dir: &Path) -> Result<Vec<WorktreeEntry>>` —
      parses `git worktree list --porcelain`, filters to entries whose path is
      under `pool_dir`, returns structured entries with path + locked flag
- [ ] 3.2 Implement `is_clean(slot_path: &Path) -> Result<bool>` — runs
      `git -C <slot-path> status --porcelain`; returns `true` if output is empty
- [ ] 3.3 Implement `prune_worktrees() -> Result<()>` — runs
      `git worktree prune`
- [ ] 3.4 Implement
      `find_available_slot(pool_dir: &Path) -> Result<Option<PathBuf>>` — calls
      `prune_worktrees()`, then scans pool entries: returns the first slot that
      exists on disk, is not locked, and is clean

## 4. Worktree Module — Provision

- [ ] 4.1 Implement `reset_slot(slot_path: &Path, head_sha: &str) -> Result<()>`
      — runs `git -C <slot-path> checkout --detach <head_sha>`
- [ ] 4.2 Implement
      `create_slot(slot_path: &Path, head_sha: &str) -> Result<()>` — runs
      `git worktree add --detach <slot-path> <head_sha>`
- [ ] 4.3 Implement `get_worktree() -> Result<PathBuf>` — orchestrates the full
      flow: resolve HEAD → resolve pool dir → **`fs::create_dir_all` the pool
      dir** → find available slot → reset it or create new → return path

## 5. CLI Wiring

- [ ] 5.1 Add `Get` variant to `Commands` enum (no options) with clap doc
      comments
- [ ] 5.2 Change `command: Commands` to `command: Option<Commands>` on `Cli`;
      remove `arg_required_else_help`
- [ ] 5.3 In `main()`, match `None` → dispatch to `get_worktree()`
- [ ] 5.4 In `main()`, match `Some(Commands::Get)` → call `get_worktree()`,
      print the path to stdout, exit 0
- [ ] 5.5 Ensure `Commands::Help` still works correctly after the
      `Option<Commands>` refactor

## 6. Error Handling & Output

- [ ] 6.1 Print the resolved absolute worktree path to stdout on success (sole
      output)
- [ ] 6.2 Print human-readable errors to stderr and exit with a non-zero code on
      all failure paths
- [ ] 6.3 Add a specific error when home dir cannot be resolved (directing user
      to ensure `$HOME` is set)
- [ ] 6.4 Add a specific error for bare repo detection (`git_common_dir()`
      returns `.`)

## 7. Tests

- [ ] 7.1 Unit test `repo_slug()` called from a path that simulates a linked
      worktree (verify it returns the main repo's name, not the slot name)
- [ ] 7.2 Unit test `new_slot_path()` output matches
      `<pool_dir>/<8-char-alphanumeric>/`
- [ ] 7.3 Unit test `find_available_slot()` returns `None` when all slots are
      dirty or locked
- [ ] 7.4 Unit test `find_available_slot()` returns the first clean, unlocked
      slot path
- [ ] 7.5 Integration / smoke test: `bs get` creates a slot under
      `~/.bonsai/<repo-slug>/`, prints path, exits 0; second call reuses it
- [ ] 7.6 Integration / smoke test: `bs` (no subcommand) behaves identically to
      `bs get`
