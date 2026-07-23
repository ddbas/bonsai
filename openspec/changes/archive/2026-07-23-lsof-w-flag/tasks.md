## 1. Add `-w` flag to `lsof` invocations

- [x] 1.1 In `run_lsof` (`src/worktree/mod.rs`), change the
      `Command::new(lsof_bin)` args from `["+d", ...]` to `["-w", "+d", ...]`.
- [x] 1.2 In `run_lsof_count` (`src/worktree/mod.rs`), apply the same
      `["-w", "+d", ...]` args change.

## 2. Remove custom warning-filtering logic

- [x] 2.1 Delete the `lsof_real_errors` function and its doc comment in
      `src/worktree/mod.rs`.
- [x] 2.2 In `run_lsof`, use `stderr` directly (trimmed) instead of calling
      `lsof_real_errors`, updating the surrounding comments to reflect that `-w`
      prevents warnings rather than post-hoc filtering.
- [x] 2.3 In `run_lsof_count`, apply the same change: use `stderr` directly
      instead of calling `lsof_real_errors`.
- [x] 2.4 Update the doc comments on `has_open_files`/`count_open_processes`
      (and any references elsewhere in the file) that describe the Docker
      overlay2/nsfs and macOS mount-table warning filtering, since that behavior
      no longer exists.

## 3. Remove obsolete tests

- [x] 3.1 Delete `lsof_real_errors_strips_docker_warnings`,
      `lsof_real_errors_strips_macos_mount_table_assumption`,
      `lsof_real_errors_preserves_real_errors`, and
      `lsof_real_errors_mixed_keeps_real_error_only` tests in
      `src/worktree/mod.rs`.

## 4. Verify

- [x] 4.1 Run `cargo build` and `cargo test` to confirm the crate compiles and
      existing `has_open_files`/`count_open_processes` tests still pass.
- [x] 4.2 Run `cargo clippy` to confirm no unused-code warnings remain after
      removing `lsof_real_errors`.
