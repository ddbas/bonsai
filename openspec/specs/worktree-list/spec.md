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

### Requirement: Each worktree is shown on its own line with path, branch, status, and usage stats

`bs list` SHALL print one line per managed pool worktree. Each line SHALL
contain:

1. A coloured status badge (`available` in green, `in use` in red).
2. The worktree path (with home directory prefix replaced with `~`).
3. Optionally, the checked-out branch name in **bold parentheses** immediately
   after the path (omitted for detached HEAD).
4. A compact usage-stats column that shows only non-zero values for: open
   processes (`⚙N`), uncommitted files (`±N`), and untracked files (`?N`),
   space-separated. The column is blank for clean idle slots.

#### Scenario: Single available worktree, detached HEAD

- **WHEN** the pool contains one slot that is clean, unlocked, has no open file
  handles, and is in detached HEAD state
- **THEN** stdout SHALL contain one line with a green `available` badge, the
  tilde-prefixed path, no branch suffix, and a blank stats column

#### Scenario: Single available worktree with a branch

- **WHEN** the pool contains one clean, unlocked, idle slot with branch `main`
  checked out
- **THEN** stdout SHALL contain one line with a green `available` badge, the
  tilde-prefixed path followed by `(main)` in bold, and a blank stats column

#### Scenario: In-use worktree with open file handles

- **WHEN** a slot has 2 open processes
- **THEN** the stats column SHALL contain `⚙2`

#### Scenario: In-use worktree with uncommitted changes only

- **WHEN** a slot has 3 modified/staged files and no open processes or untracked
  files
- **THEN** the stats column SHALL contain `±3`

#### Scenario: In-use worktree with all three stats

- **WHEN** a slot has 1 open process, 2 uncommitted files, and 3 untracked files
- **THEN** the stats column SHALL be `⚙1 ±2 ?3`

#### Scenario: Mixed pool

- **WHEN** the pool contains multiple slots in different states
- **THEN** each slot SHALL appear on its own line with the correct badge, path,
  optional branch, and stats column

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
