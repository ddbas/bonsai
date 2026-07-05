## ADDED Requirements

### Requirement: `count_open_processes` returns the number of distinct PIDs with open handles

The system SHALL provide a `count_open_processes(path: &Path) -> Result<usize>`
function in the `worktree` module. The function SHALL run `lsof +D <path>`,
parse the PID field (second whitespace-delimited column) from each non-header
output line, deduplicate the PIDs, and return the count. It SHALL return `0`
when `lsof` exits with no matches (exit code 1, empty stdout).

#### Scenario: Multiple file descriptors from one process

- **WHEN** `lsof +D <path>` lists three rows all with the same PID
- **THEN** `count_open_processes` SHALL return `Ok(1)`

#### Scenario: Multiple distinct processes

- **WHEN** `lsof +D <path>` lists rows with PIDs 100, 200, and 100
- **THEN** `count_open_processes` SHALL return `Ok(2)`

#### Scenario: No open file handles

- **WHEN** `lsof` exits with code 1 and empty stdout
- **THEN** `count_open_processes` SHALL return `Ok(0)`

#### Scenario: `lsof` not found

- **WHEN** `lsof` is not present on `PATH`
- **THEN** `count_open_processes` SHALL return `Err` whose message names `lsof`
  as the missing dependency

### Requirement: `list_worktrees_status` exposes per-slot process count

`list_worktrees_status` SHALL return
`Vec<(PathBuf, WorktreeStatus, Option<usize>)>`. The third element SHALL be
`Some(n)` where `n > 0` when the slot is `InUse` due to open file handles, and
`None` in all other cases (available, locked, dirty, or when `lsof` returns an
error that is propagated).

#### Scenario: Available slot has `None` count

- **WHEN** a slot is `WorktreeStatus::Available`
- **THEN** the process-count element SHALL be `None`

#### Scenario: Locked slot has `None` count

- **WHEN** a slot is `WorktreeStatus::InUse` because it is git-locked
- **THEN** the process-count element SHALL be `None`

#### Scenario: Open-file slot has `Some(n)` count

- **WHEN** a slot is `WorktreeStatus::InUse` because `count_open_processes`
  returned `n > 0`
- **THEN** the process-count element SHALL be `Some(n)`
