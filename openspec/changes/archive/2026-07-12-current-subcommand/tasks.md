## 1. Library function

- [x] 1.1 Add `current_worktree() -> Result<Option<(PathBuf, Option<String>)>>`
      to `src/worktree/mod.rs` that resolves the CWD, builds the pool directory,
      and prefix-matches against pool entries from `list_pool_worktrees`
- [x] 1.2 Return `Ok(None)` (not an error) when the pool directory does not yet
      exist
- [x] 1.3 Canonicalise the CWD before comparing to pool entry paths (mirrors
      `get_worktree` handling for macOS `/tmp` → `/private/tmp` symlinks)

## 2. CLI wiring

- [x] 2.1 Add `Current` variant to the `Commands` enum in `src/main.rs` with doc
      comment matching the TODO description
- [x] 2.2 Add the `current` match arm: call `worktree::current_worktree()`,
      print `🌳 <path>` or `🌳 <path>  (<branch>)` on success, print
      informational message and call `std::process::exit(1)` when `Ok(None)`

## 3. Unit tests

- [x] 3.1 Test `current_worktree` returns `Ok(None)` when pool dir is absent
      (use `BONSAI_ROOT` env override pointing to a non-existent dir)
- [x] 3.2 Test the `format_stats`-style helper: path output is `<tilde>` with no
      branch when branch is `None`, and `<tilde> (<branch>)` when branch is
      `Some`

## 4. Integration / manual verification

- [x] 4.1 Run `cargo build` and verify `bs current --help` shows the new
      subcommand
- [x] 4.2 From inside a slot provisioned by `bs get -b test-branch`, verify
      `bs current` prints the slot path with `(test-branch)` and exits 0
- [x] 4.3 From the main repo directory (not a pool slot), verify `bs current`
      prints an informational message and exits 1
- [x] 4.4 Run `cargo test` and confirm all existing and new unit tests pass
