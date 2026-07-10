## Purpose

Provide `has_open_files` and `count_open_processes` helpers that detect whether
any process currently holds open file descriptors in the top-level directory of
a managed worktree pool slot, enabling safe availability classification before a
slot is reused.

## Requirements

### Requirement: `has_open_files` detects open file handles in the top-level slot directory

The system SHALL provide a `has_open_files(path: &Path) -> Result<bool>`
function in the `worktree` module. The function SHALL run `lsof +d <path>`
(non-recursive, top-level directory only) and return `true` when at least one
process has an open file descriptor **directly in** `path` (including processes
whose current working directory is `path`), and `false` when no process does.

The function SHALL NOT scan files in subdirectories of `path`. A process with
only open file descriptors in subdirectories of `path` (but not in `path`
itself) SHALL NOT cause `has_open_files` to return `true`.

#### Scenario: Shell has slot directory as CWD

- **WHEN** a shell process has its current working directory set to `path`
- **THEN** `has_open_files` SHALL return `Ok(true)`

#### Scenario: Process has file open directly in slot directory

- **WHEN** at least one process has an open file descriptor to a file located
  directly inside `path` (not in a subdirectory)
- **THEN** `has_open_files` SHALL return `Ok(true)`

#### Scenario: Process has file open only in subdirectory

- **WHEN** a process has an open file descriptor only in a subdirectory of
  `path` (e.g. `<path>/src/main.rs`) and no open handle in `path` itself
- **THEN** `has_open_files` SHALL return `Ok(false)`

#### Scenario: No process has files open

- **WHEN** no process has any open file descriptor in `path`
- **THEN** `has_open_files` SHALL return `Ok(false)`

### Requirement: `lsof` absence causes the CLI to exit with a hard error

The system SHALL return `Err(...)` from `has_open_files` when `lsof` cannot be
spawned. The error message SHALL clearly identify `lsof` as the missing
dependency and include a hint for installing it (e.g. `brew install lsof`). The
error SHALL propagate to `main` unmodified, causing the CLI to exit with a
non-zero status and print the message to stderr.

#### Scenario: `lsof` not found

- **WHEN** `lsof` is not present on `PATH`
- **THEN** `has_open_files` SHALL return an `Err` whose message names `lsof` as
  the missing dependency
- **THEN** `bs get` or `bs list` SHALL exit with a non-zero status and print the
  error to stderr

### Requirement: `lsof` failure on a specific path causes `has_open_files` to return an error

The system SHALL return `Err(...)` from `has_open_files` when `lsof` exits with
a non-zero status AND produces non-empty stderr output, indicating a real error
rather than "no matches".

#### Scenario: `lsof` exits with error output

- **WHEN** `lsof` exits non-zero and writes to stderr
- **THEN** `has_open_files` SHALL return an `Err` value

### Requirement: A slot with open file handles at the slot root is classified as `InUse`

`list_worktrees_status` and `find_available_slot` SHALL treat a slot as
`WorktreeStatus::InUse` when `has_open_files` returns `Ok(true)` for that slot,
even if the slot is otherwise unlocked and has a clean working tree.

A process whose open handles are only in subdirectories of the slot root SHALL
NOT trigger the `InUse` classification via `has_open_files`.

#### Scenario: Shell is cd'd to slot root

- **WHEN** a slot is unlocked, clean, and exists on disk
- **WHEN** a shell process has CWD = the slot root directory
- **THEN** the slot SHALL be classified as `WorktreeStatus::InUse`

#### Scenario: Process has handle only in subdirectory

- **WHEN** a slot is unlocked, clean, and exists on disk
- **WHEN** a process has an open file descriptor only in a subdirectory of the
  slot
- **THEN** `has_open_files` SHALL return `Ok(false)` and the slot SHALL NOT be
  classified as `InUse` on that basis alone

### Requirement: A slot with no open file handles may still be classified as `Available`

`list_worktrees_status` and `find_available_slot` SHALL classify a slot as
`WorktreeStatus::Available` only when it passes all four preconditions: exists
on disk, not git-locked, working tree is clean, and `has_open_files` returns
`Ok(false)`.

#### Scenario: Clean unlocked slot with no open handles at root

- **WHEN** a slot is unlocked, clean, exists on disk, and no process has a
  handle directly in its root directory
- **THEN** the slot SHALL be classified as `WorktreeStatus::Available`

### Requirement: `has_open_files` errors propagate as hard failures

When `has_open_files` returns `Err`, the caller SHALL propagate the error up the
call stack rather than swallowing it. No slot SHALL be silently skipped or
treated as `InUse` when the underlying failure is a missing `lsof` binary — the
CLI MUST stop and report the problem.

#### Scenario: `lsof` not installed — CLI fails with actionable message

- **WHEN** `lsof` cannot be spawned
- **THEN** `find_available_slot` SHALL return `Err` immediately
- **THEN** `bs get` SHALL exit non-zero and print an error naming `lsof` as the
  missing dependency to stderr

#### Scenario: `lsof` runtime error propagates

- **WHEN** `lsof` exits non-zero with non-empty stderr
- **THEN** `has_open_files` SHALL return `Err`
- **THEN** the CLI SHALL propagate the error and exit non-zero

### Requirement: `count_open_processes` returns the number of distinct PIDs with open handles in the top-level slot directory

The system SHALL provide a `count_open_processes(path: &Path) -> Result<usize>`
function in the `worktree` module. The function SHALL run `lsof +d <path>`
(non-recursive), parse the PID field (second whitespace-delimited column) from
each non-header output line, deduplicate the PIDs, and return the count. It
SHALL return `0` when `lsof` exits with no matches.

The function SHALL NOT count processes that have open handles only in
subdirectories of `path`.

#### Scenario: Shell is cd'd to slot root

- **WHEN** exactly one shell process has CWD = `path`
- **THEN** `count_open_processes` SHALL return `Ok(1)`

#### Scenario: Multiple file descriptors from one process in top-level dir

- **WHEN** `lsof +d <path>` lists three rows all with the same PID
- **THEN** `count_open_processes` SHALL return `Ok(1)`

#### Scenario: Multiple distinct processes with top-level handles

- **WHEN** `lsof +d <path>` lists rows with PIDs 100, 200, and 100
- **THEN** `count_open_processes` SHALL return `Ok(2)`

#### Scenario: No open file handles in top-level dir

- **WHEN** `lsof +d` reports no matches
- **THEN** `count_open_processes` SHALL return `Ok(0)`

#### Scenario: Process only has files in subdirectory

- **WHEN** a process has an open handle only at `<path>/src/main.rs`
- **THEN** `count_open_processes` SHALL return `Ok(0)`

#### Scenario: `lsof` not found

- **WHEN** `lsof` is not present on `PATH`
- **THEN** `count_open_processes` SHALL return `Err` whose message names `lsof`
  as the missing dependency
