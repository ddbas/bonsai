## 1. Core Implementation — `has_open_files` helper

- [x] 1.1 Add `has_open_files(path: &Path) -> Result<bool>` to
      `src/worktree/mod.rs` that runs `lsof +D <path>` and returns `true` when
      stdout is non-empty
- [x] 1.2 Handle exit-code semantics: exit 0 + stdout → `Ok(true)`; exit 1 +
      empty stderr → `Ok(false)`; spawn error (lsof not found) → `Err` with
      message
      `"lsof not found on PATH — install lsof to use bs (e.g. brew install lsof)"`;
      non-empty stderr → `Err` propagating the lsof output
- [x] 1.3 Add doc comment explaining the `lsof +D` approach and the hard-fail
      contract when `lsof` is absent

## 2. Integrate check into availability classification — propagate errors

- [x] 2.1 Update `find_available_slot` to call `has_open_files` after
      `is_clean`; propagate `Err` immediately (do not swallow it as InUse)
- [x] 2.2 Update `list_worktrees_status` to call `has_open_files`; map
      `Ok(true)` to `WorktreeStatus::InUse` and propagate `Err` to the caller
- [x] 2.3 Verify the ordering: locked → not exists → dirty → open files →
      Available (short-circuit on first failing condition)

## 3. Unit tests for `has_open_files`

- [x] 3.1 Test: create a temp file, hold it open with `File::open`, assert
      `has_open_files(dir)` returns `Ok(true)` (integration-style unit test, no
      Docker needed since it's in-process)
- [x] 3.2 Test: create a temp dir with no open handles, assert
      `has_open_files(dir)` returns `Ok(false)`
- [x] 3.3 Test: when `lsof` is unavailable (override `PATH` to empty dir),
      assert `has_open_files` returns `Err` whose message mentions `lsof`
- [x] 3.4 Test (integration): `bs get` exits non-zero and stderr contains "lsof"
      when `lsof` is not on PATH

## 4. Update synthetic status tests

- [x] 4.1 Update `synthetic_status` helper in existing unit tests to accept a
      fifth `open_files: bool` parameter
- [x] 4.2 Add new test `status_open_files_is_in_use`: unlocked + exists +
      clean + open handles → `InUse`
- [x] 4.3 Confirm existing `status_clean_unlocked_is_available` test now also
      passes `open_files: false`

## 5. Spec updates in `openspec/specs/`

- [x] 5.1 Archive the delta spec for `worktree-get`: apply MODIFIED requirement
      into `openspec/specs/worktree-get/spec.md`
- [x] 5.2 Archive the delta spec for `worktree-list`: apply MODIFIED requirement
      into `openspec/specs/worktree-list/spec.md`

## 6. Documentation

- [x] 6.1 Add `lsof` to the **Prerequisites** section of `README.md` as a
      required runtime dependency
