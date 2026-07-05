## Purpose

Provide a `has_open_files` helper that detects whether any process currently
holds open file descriptors inside a managed worktree pool slot directory,
enabling safe availability classification before a slot is reused.

## Requirements

### Requirement: `has_open_files` detects open file handles inside a slot directory

The system SHALL provide a `has_open_files(path: &Path) -> Result<bool>`
function in the `worktree` module. The function SHALL run `lsof +D <path>` and
return `true` when at least one process has an open file descriptor anywhere
inside `path` (including `path` itself), and `false` when no process does.

#### Scenario: Process has file open inside directory

- **WHEN** at least one process has an open file descriptor under `path`
- **THEN** `has_open_files` SHALL return `Ok(true)`

#### Scenario: No process has files open

- **WHEN** no process has any open file descriptor under `path`
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

### Requirement: A slot with open file handles is classified as `InUse`

`list_worktrees_status` and `find_available_slot` SHALL treat a slot as
`WorktreeStatus::InUse` when `has_open_files` returns `Ok(true)` for that slot,
even if the slot is otherwise unlocked and has a clean working tree.

#### Scenario: Editor has slot file open

- **WHEN** a slot is unlocked, clean, and exists on disk
- **WHEN** an editor process has a file in that slot open
- **THEN** the slot SHALL be classified as `WorktreeStatus::InUse`

### Requirement: A slot with no open file handles may still be classified as `Available`

`list_worktrees_status` and `find_available_slot` SHALL classify a slot as
`WorktreeStatus::Available` only when it passes all four preconditions: exists
on disk, not git-locked, working tree is clean, and `has_open_files` returns
`Ok(false)`.

#### Scenario: Clean unlocked slot with no open handles

- **WHEN** a slot is unlocked, clean, exists on disk, and no process has it open
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
