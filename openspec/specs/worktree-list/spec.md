## Purpose

Provide a `bs list` subcommand (alias `bs ls`) that enumerates all
bonsai-managed worktrees for the current repository's pool and displays each
slot with a colour-coded availability badge, making it easy to inspect pool
state without parsing raw git output.

## Requirements

### Requirement: `list` subcommand exists with `ls` alias

The CLI SHALL expose a `list` subcommand. Running `bs list` SHALL be equivalent
to running `bs ls`.

#### Scenario: Invoke with `list`

- **WHEN** the user runs `bs list`
- **THEN** the process exits with code 0 and prints the worktree list to stdout

#### Scenario: Invoke with `ls` alias

- **WHEN** the user runs `bs ls`
- **THEN** the process behaves identically to `bs list`

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

### Requirement: Available status means clean, unlocked, and not opened by any process

A worktree slot SHALL be displayed as **available** (green) if and only if: (1)
its directory exists on disk, (2) it is not locked, (3) its working tree is
clean (`git -C <slot> status --porcelain` returns empty output), and (4) no
process currently has an open file handle anywhere inside the slot directory (as
determined by `lsof +D <slot>`). If `lsof` cannot be run, the CLI SHALL exit
with a non-zero status and an actionable error message.

#### Scenario: Locked slot shown as in use

- **WHEN** a pool slot is locked
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Dirty slot shown as in use

- **WHEN** a pool slot has uncommitted changes
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Slot with open file handles shown as in use

- **WHEN** a pool slot is unlocked and clean
- **WHEN** a process has a file open inside the slot directory
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Clean unlocked slot with no open handles shown as available

- **WHEN** a pool slot is unlocked, its working tree is clean, and no process
  has it open
- **THEN** `bs list` SHALL display it with a green `available` badge

### Requirement: Empty pool prints a friendly message

If no worktree slots exist in the pool, `bs list` SHALL print a human-readable
message to stdout indicating the pool is empty rather than printing nothing or
returning an error.

#### Scenario: Pool directory does not exist

- **WHEN** `~/.bonsai/<repo-slug>/` does not exist
- **THEN** `bs list` exits with code 0 and prints a message indicating no
  worktrees are managed for this repository

#### Scenario: Pool directory exists but is empty

- **WHEN** `~/.bonsai/<repo-slug>/` exists but contains no registered worktrees
- **THEN** `bs list` exits with code 0 and prints a message indicating no
  worktrees are managed for this repository

### Requirement: Home directory prefix is displayed as `~`

Paths displayed by `bs list` SHALL have the user's home directory prefix
replaced with `~` for readability.

#### Scenario: Path under home directory

- **WHEN** a slot path is `/Users/alice/.bonsai/myrepo/a3f9c1b2`
- **WHEN** the home directory is `/Users/alice`
- **THEN** the displayed path SHALL be `~/.bonsai/myrepo/a3f9c1b2`

#### Scenario: Path not under home directory

- **WHEN** a slot path does not start with the home directory
- **THEN** the full absolute path SHALL be displayed unchanged

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
