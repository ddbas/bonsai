## MODIFIED Requirements

### Requirement: Each worktree is shown on its own line with path, status, and optional process count

`bs list` SHALL print one line per managed pool worktree. Each line SHALL
contain a coloured status badge, the worktree path (with home directory prefix
replaced with `~`), and an optional process-count column. The process-count
column SHALL be printed only when the slot has one or more processes with open
file handles inside it; otherwise the column SHALL be blank.

#### Scenario: Single available worktree

- **WHEN** the pool contains one slot that is clean, unlocked, and has no open
  file handles
- **THEN** stdout SHALL contain one line with a green `available` badge, the
  tilde-prefixed path, and a blank process-count column

#### Scenario: Single in-use worktree with no open file handles

- **WHEN** the pool contains one slot that is locked, or has uncommitted changes
  but no open file handles
- **THEN** stdout SHALL contain one line with a red `in use` badge, the
  tilde-prefixed path, and a blank process-count column

#### Scenario: In-use worktree with open file handles shows process count

- **WHEN** the pool contains one slot that is in use because processes have
  files open inside it
- **WHEN** exactly N distinct processes hold open file handles in that slot
- **THEN** stdout SHALL contain one line with a red `in use` badge, the
  tilde-prefixed path, and the number N in the process-count column

#### Scenario: Dirty worktree with open file handles shows process count

- **WHEN** the pool contains one slot that has both uncommitted changes AND open
  file handles from N distinct processes
- **THEN** stdout SHALL contain one line with a red `in use` badge, the
  tilde-prefixed path, and the number N in the process-count column
- **NOTE** The presence of uncommitted changes SHALL NOT suppress the count

#### Scenario: Process count column is blank for available slots

- **WHEN** a slot is available (clean, unlocked, no open handles)
- **THEN** the process-count column for that slot SHALL be blank (not `0`)

#### Scenario: Mixed pool

- **WHEN** the pool contains multiple slots in different states
- **THEN** each slot SHALL appear on its own line with the correct badge, path,
  and process count (blank where not applicable)

## ADDED Requirements

### Requirement: Process count column displays the number of distinct open-file processes

The process-count value shown in `bs list` SHALL be the number of **distinct
PIDs** that have at least one open file descriptor anywhere inside the slot
directory, as reported by `lsof +D <slot>`. It SHALL NOT count file descriptors
— one process with five open files in the slot counts as 1, not 5.

#### Scenario: One process with multiple open files

- **WHEN** one process has three files open inside a slot
- **THEN** the process-count column SHALL show `1`

#### Scenario: Three distinct processes

- **WHEN** three different processes each have at least one file open inside a
  slot
- **THEN** the process-count column SHALL show `3`
