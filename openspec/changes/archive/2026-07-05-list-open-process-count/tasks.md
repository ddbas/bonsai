## 1. Core helper — `count_open_processes`

- [x] 1.1 Add `count_open_processes(path: &Path) -> Result<usize>` to
      `src/worktree/mod.rs` that runs `lsof +D <path>` and parses the PID field
      (second whitespace-delimited column) from each non-header line
- [x] 1.2 Deduplicate parsed PIDs with a `HashSet` and return the count; return
      `Ok(0)` when `lsof` exits non-zero with empty stdout (no matches)
- [x] 1.3 Propagate `Err` with a message naming `lsof` when the binary is not
      found (consistent with `has_open_files` contract)
- [x] 1.4 Replace the `has_open_files` call site in `find_available_slot` with
      `count_open_processes`; treat `count > 0` as "has open files"

## 2. Update `list_worktrees_status` return type

- [x] 2.1 Change return type from `Vec<(PathBuf, WorktreeStatus)>` to
      `Vec<(PathBuf, WorktreeStatus, Option<usize>)>`
- [x] 2.2 Set the third element to `Some(n)` (where `n > 0`) when the slot is
      `InUse` due to open file handles; `None` for all other cases (available,
      locked, dirty, or error)
- [x] 2.3 Use a single `count_open_processes` call per slot — derive both the
      `WorktreeStatus` and the count from the same result

## 3. Update `bs list` rendering

- [x] 3.1 Destructure the new three-tuple in the `Commands::List` match arm in
      `src/main.rs`
- [x] 3.2 When `process_count` is `Some(n)`, append a right-aligned count column
      (e.g. `format!("{:>4} processes", n)`) after the path
- [x] 3.3 When `process_count` is `None`, leave the column blank (no trailing
      text)
- [x] 3.4 Verify alignment is consistent across mixed-state pool output
      (available + in-use rows line up)

## 4. Unit tests for `count_open_processes`

- [x] 4.1 Test: hold a file open in a temp dir, assert `count_open_processes`
      returns `Ok(1)`
- [x] 4.2 Test: temp dir with no open handles, assert returns `Ok(0)`
- [x] 4.3 Test: parse correctness — feed mock lsof output with duplicate PIDs,
      assert deduplication returns the right count
- [x] 4.4 Test: `lsof` not on PATH (empty PATH dir), assert `Err` whose message
      mentions `lsof`

## 5. Update existing tests for the changed return type

- [x] 5.1 Update any call sites of `list_worktrees_status` in tests to
      destructure the new three-tuple
- [x] 5.2 Update `synthetic_status` helper (unit tests) to include the
      `Option<usize>` count field
- [x] 5.3 Add assertion: available slot tuple has `None` as third element
- [x] 5.4 Add assertion: in-use-due-to-open-files slot tuple has `Some(n)` as
      third element
