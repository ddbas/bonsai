## MODIFIED Requirements

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
