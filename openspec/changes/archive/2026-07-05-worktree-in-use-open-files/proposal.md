## Why

The current "available vs in use" heuristic (locked + dirty working tree) misses
a critical case: another process — an editor, a build tool, a shell — has the
worktree directory (or a file inside it) open. Handing that slot to a new task
while a process still holds a handle can lead to subtle corruption, unexpected
build failures, or file-system races. Detecting open file handles before reusing
a slot makes the pool safe in real-world multi-process workflows.

## What Changes

- Add an `is_process_using` check that uses `lsof` to detect whether any process
  has an open file descriptor inside a slot directory.
- Extend the `WorktreeStatus::Available` precondition: a slot is available
  **only if** no process has the directory (or any file within it) open.
- Update `find_available_slot` and `list_worktrees_status` to incorporate the
  new check.
- Add a `has_open_files` public helper to `worktree` so it is testable and
  reusable.
- Update both the `worktree-get` and `worktree-list` specs to document the new
  precondition for "available".

## Capabilities

### New Capabilities

- `worktree-open-file-detection`: Detect whether any process has open file
  handles inside a pool worktree slot directory, using `lsof +D <path>`.

### Modified Capabilities

- `worktree-get`: The "available slot" definition gains a third precondition: no
  process has open file handles inside the slot.
- `worktree-list`: The `available` display badge precondition gains the same
  third condition.

## Impact

- **`src/worktree/mod.rs`**: new `has_open_files(path)` function; updated
  `find_available_slot` and `list_worktrees_status`.
- **`openspec/specs/worktree-get/spec.md`**: amended availability definition.
- **`openspec/specs/worktree-list/spec.md`**: amended availability definition.
- **Dependencies**: no new crate dependencies — `lsof` is invoked as an external
  process (available on macOS and most Linux distros; behaviour on systems
  without `lsof` is graceful fallback to "in use").
- **Tests**: new unit/integration tests for `has_open_files`; updated
  synthetic-status helper in existing tests.
