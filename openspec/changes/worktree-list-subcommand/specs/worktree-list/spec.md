## ADDED Requirements

### Requirement: `list` subcommand exists with `ls` alias

The CLI SHALL expose a `list` subcommand. Running `bs list` SHALL be equivalent
to running `bs ls`.

#### Scenario: Invoke with `list`

- **WHEN** the user runs `bs list`
- **THEN** the process exits with code 0 and prints the worktree list to stdout

#### Scenario: Invoke with `ls` alias

- **WHEN** the user runs `bs ls`
- **THEN** the process behaves identically to `bs list`

### Requirement: Each worktree is shown on its own line with path and status

`bs list` SHALL print one line per managed pool worktree. Each line SHALL
contain a coloured status badge followed by the worktree path. The path SHALL
have the home directory prefix replaced with `~`.

#### Scenario: Single available worktree

- **WHEN** the pool contains one slot that is clean and unlocked
- **THEN** stdout SHALL contain one line with a green `available` badge and the
  tilde-prefixed path of that slot

#### Scenario: Single in-use worktree

- **WHEN** the pool contains one slot that is locked or has uncommitted changes
- **THEN** stdout SHALL contain one line with a red `in use` badge and the
  tilde-prefixed path of that slot

#### Scenario: Mixed pool

- **WHEN** the pool contains multiple slots in different states
- **THEN** each slot SHALL appear on its own line with the correct colour and
  badge

### Requirement: Available status means clean and unlocked

A worktree slot SHALL be displayed as **available** (green) if and only if: (1)
its directory exists on disk, (2) it is not locked, and (3) its working tree is
clean (`git -C <slot> status --porcelain` returns empty output).

#### Scenario: Locked slot shown as in use

- **WHEN** a pool slot is locked
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Dirty slot shown as in use

- **WHEN** a pool slot has uncommitted changes
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Clean unlocked slot shown as available

- **WHEN** a pool slot is unlocked and its working tree is clean
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
