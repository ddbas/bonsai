## 1. Config reading

- [x] 1.1 Add a helper in `src/worktree/mod.rs` (e.g. `configured_copy_paths`)
      that runs `git config --get-all bonsai.copy` from a given directory (the
      origin worktree) via the existing `git_cmd()` helper and returns
      `Vec<String>` of configured entries, preserving order
- [x] 1.2 Treat a non-zero/empty-result exit from `git config --get-all` (key
      not set) as an empty list rather than an error, while still surfacing
      genuine git errors (e.g. corrupt config)
- [x] 1.3 Unit test: no `bonsai.copy` set returns an empty `Vec`
- [x] 1.4 Unit test: single `bonsai.copy` entry is parsed correctly
- [x] 1.5 Unit test: multiple `bonsai.copy` entries (via repeated `--add`) are
      returned in config order

## 2. File copy helper

- [x] 2.1 Add a helper (e.g. `copy_ignored_files`) that takes the origin
      worktree root, the destination slot path, and the list of relative paths,
      and copies each existing source file to the corresponding destination path
- [x] 2.2 Create destination parent directories as needed (`create_dir_all`)
      before copying each file
- [x] 2.3 Skip silently (no error) when a source file does not exist; propagate
      errors for genuine copy failures (e.g. permission denied)
- [x] 2.4 Unit test: copying a file that exists succeeds and destination content
      matches source
- [x] 2.5 Unit test: a missing source file is skipped without error, and
      remaining entries still get processed
- [x] 2.6 Unit test: destination subdirectory that doesn't exist is created
      automatically
- [x] 2.7 Unit test: empty file list is a no-op

## 3. Wire into `get_worktree`

- [x] 3.1 Capture the origin worktree's current working directory at the start
      of `get_worktree` (before any slot path changes)
- [x] 3.2 After the slot is created (`create_slot`) or reset (`reset_slot`) and
      before the final `canonicalize`/return, read `bonsai.copy` from the origin
      worktree and call the copy helper with (origin root, slot path, entries)
- [x] 3.3 Ensure the copy step runs identically for both the "reuse existing
      slot" and "create new slot" branches in `get_worktree`
- [x] 3.4 Add `tracing` debug/info logging for the copy step (number of files
      copied, entries skipped), consistent with existing logging style in the
      module

## 4. Integration tests

- [x] 4.1 Add a new test file (e.g. `tests/worktree_copy_ignored_files.rs`)
      following the conventions in `tests/worktree_get.rs` /
      `tests/common/mod.rs`
- [x] 4.2 Test: `bonsai.copy` configured with a file present in the origin
      worktree results in that file existing in the newly created slot after
      `bs get`
- [x] 4.3 Test: same as above but for a slot that is reused (already exists and
      is available) rather than newly created
- [x] 4.4 Test: `bonsai.copy` listing a nonexistent file does not cause `bs get`
      to fail, and other configured files are still copied
- [x] 4.5 Test: with `bonsai.copy` unset, `bs get` behavior and output are
      unchanged (regression guard)
- [x] 4.6 Test: `bonsai.copy` entry with a nested relative path (e.g.
      `config/local.json`) is copied with parent directories created in the slot

## 5. Documentation

- [x] 5.1 Document the `bonsai.copy` git config key (namespace, multi-value
      usage, example `git config --add bonsai.copy .env`) in the project README
      or CLI help text, matching how other bonsai behaviors are documented
- [x] 5.2 Note the silent-skip-on-missing-file behavior in the documentation so
      it isn't mistaken for a bug

## 6. Validation

- [x] 6.1 Run `cargo fmt` and
      `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 6.2 Run the full test suite (`cargo test`) and confirm all new and
      existing tests pass
- [x] 6.3 Manually verify with a real `bonsai.copy` config against a throwaway
      repo: reused slot, new slot, and missing-file scenarios
