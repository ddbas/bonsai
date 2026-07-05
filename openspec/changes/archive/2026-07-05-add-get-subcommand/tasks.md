## 1. Dependencies & Project Setup

- [x] 1.1 Add `dirs = "5"` to `[dependencies]` in `Cargo.toml`
- [x] 1.2 Add `uuid = { version = "1", features = ["v4"] }` to `[dependencies]`
      in `Cargo.toml`
- [x] 1.3 Verify project builds cleanly after adding dependencies
      (`mise run build`)

## 2. Worktree Module — Core Utilities

- [x] 2.1 Create `src/worktree/mod.rs` and declare the module in `src/lib.rs`
- [x] 2.2 Implement `git_common_dir() -> Result<PathBuf>` — runs
      `git rev-parse --git-common-dir` in the current directory; errors on bare
      repos
- [x] 2.3 Implement `repo_slug() -> Result<String>` — calls `git_common_dir()`,
      takes its parent as the main repo root, returns its basename lowercased
      with non-alphanumeric chars replaced by `-`
- [x] 2.4 Implement `resolve_head() -> Result<String>` — runs
      `git rev-parse HEAD` in the current directory; returns the full commit SHA
- [x] 2.5 Implement `managed_root() -> Result<PathBuf>` — returns
      `dirs::home_dir()/.bonsai`; errors with a clear message if home dir cannot
      be resolved
- [x] 2.6 Implement `new_slot_path(pool_dir: &Path) -> PathBuf` — generates an
      8-char UUID v4 prefix and returns `pool_dir/<prefix>`

## 3. Worktree Module — Pool Scan

- [x] 3.1 Implement
      `list_pool_worktrees(pool_dir: &Path) -> Result<Vec<WorktreeEntry>>` —
      parses `git worktree list --porcelain`, filters to entries whose path is
      under `pool_dir`, returns structured entries with path + locked flag
- [x] 3.2 Implement `is_clean(slot_path: &Path) -> Result<bool>` — runs
      `git -C <slot-path> status --porcelain`; returns `true` if output is empty
- [x] 3.3 Implement `prune_worktrees() -> Result<()>` — runs
      `git worktree prune`
- [x] 3.4 Implement
      `find_available_slot(pool_dir: &Path) -> Result<Option<PathBuf>>` — calls
      `prune_worktrees()`, then scans pool entries: returns the first slot that
      exists on disk, is not locked, and is clean

## 4. Worktree Module — Provision

- [x] 4.1 Implement `reset_slot(slot_path: &Path, head_sha: &str) -> Result<()>`
      — runs `git -C <slot-path> checkout --detach <head_sha>`
- [x] 4.2 Implement
      `create_slot(slot_path: &Path, head_sha: &str) -> Result<()>` — runs
      `git worktree add --detach <slot-path> <head_sha>`
- [x] 4.3 Implement `get_worktree() -> Result<PathBuf>` — orchestrates the full
      flow: resolve HEAD → resolve pool dir → **`fs::create_dir_all` the pool
      dir** → find available slot → reset it or create new → return path

## 5. CLI Wiring

- [x] 5.1 Add `Get` variant to `Commands` enum (no options) with clap doc
      comments
- [x] 5.2 Change `command: Commands` to `command: Option<Commands>` on `Cli`;
      remove `arg_required_else_help`
- [x] 5.3 In `main()`, match `None` → dispatch to `get_worktree()`
- [x] 5.4 In `main()`, match `Some(Commands::Get)` → call `get_worktree()`,
      print the path to stdout, exit 0
- [x] 5.5 Ensure `Commands::Help` still works correctly after the
      `Option<Commands>` refactor

## 6. Error Handling & Output

- [x] 6.1 Print the resolved absolute worktree path to stdout on success (sole
      output)
- [x] 6.2 Print human-readable errors to stderr and exit with a non-zero code on
      all failure paths
- [x] 6.3 Add a specific error when home dir cannot be resolved (directing user
      to ensure `$HOME` is set)
- [x] 6.4 Add a specific error for bare repo detection (`git_common_dir()`
      returns `.`)

## 7. Unit Tests

- [x] 7.1 `repo_slug()`: returns the main repo's basename when called from a
      path inside a linked worktree (not the slot directory name)
- [x] 7.2 `repo_slug()`: lowercases the name and replaces non-alphanumeric
      characters with `-`
- [x] 7.3 `new_slot_path()`: returned path is `<pool_dir>/<8-char-hex>/` and two
      successive calls produce different names
- [x] 7.4 `find_available_slot()`: returns `None` when every pool slot is dirty
- [x] 7.5 `find_available_slot()`: returns `None` when every pool slot is locked
- [x] 7.6 `find_available_slot()`: returns the path of the first clean, unlocked
      slot when one exists alongside dirty/locked ones
- [x] 7.7 `find_available_slot()`: returns `None` when the pool directory is
      empty

## 8. Integration Tests

- [x] 8.1 **Pool dirs created on first run**: run `bs get` against a temp root
      with no existing `~/.bonsai`-equivalent; assert both the root and
      `<repo-slug>/` subdirectory are created and the command exits 0
- [x] 8.2 **Pool dirs idempotent**: run `bs get` twice; assert no error on the
      second call when the directories already exist
- [x] 8.3 **New slot created**: run `bs get` on an empty pool; assert a
      UUID-named subdirectory is created, it is a registered git worktree, it is
      in detached HEAD state, and its path is printed to stdout
- [x] 8.4 **Existing clean slot reused**: run `bs get` twice on an empty pool;
      assert both calls return the same path and only one worktree slot exists
- [x] 8.5 **Slot reset to current HEAD**: advance HEAD by one commit; run
      `bs get`; assert the reused slot's HEAD matches the new commit SHA
- [x] 8.6 **Dirty slot skipped, new slot created**: make an uncommitted change
      inside an existing slot; run `bs get`; assert a second UUID slot is
      created and returned instead of the dirty one
- [x] 8.7 **Locked slot skipped, new slot created**: lock an existing slot with
      `git worktree lock`; run `bs get`; assert a new slot is created and
      returned
- [x] 8.8 **Stale registration pruned**: register a worktree then manually
      delete its directory; run `bs get`; assert `git worktree prune` cleans up
      the stale entry and a fresh slot is created successfully
- [x] 8.9 **Called from a linked worktree**: `cd` into an existing managed slot
      before running `bs get`; assert the repo slug is derived from the main
      repo (not the slot directory) and a valid slot path is returned
- [x] 8.10 **Default command**: run `bs` with no subcommand; assert output and
      exit code are identical to `bs get`
