## ADDED Requirements

### Requirement: Detect current managed worktree slot

The system SHALL provide a `current` subcommand that determines whether the
process's current working directory is inside a managed bonsai pool slot for the
active repository. If it is, the subcommand SHALL print the slot's
tilde-abbreviated path to stdout and exit with status 0. If it is not, the
subcommand SHALL print an informational message to stdout and exit with
status 1.

#### Scenario: CWD is the root of a managed slot

- **WHEN** the user runs `bs current` with their CWD set to a managed pool slot
  root (e.g. `~/.bonsai/repo/a3f9c1b2`)
- **THEN** the subcommand prints the tilde-abbreviated path of that slot and
  exits with status 0

#### Scenario: CWD is a subdirectory of a managed slot

- **WHEN** the user runs `bs current` with their CWD set to a subdirectory
  inside a managed pool slot (e.g. `~/.bonsai/repo/a3f9c1b2/src`)
- **THEN** the subcommand prints the tilde-abbreviated path of the containing
  slot and exits with status 0

#### Scenario: CWD is not inside any managed slot

- **WHEN** the user runs `bs current` from a directory that is not inside any
  managed pool slot for the current repository
- **THEN** the subcommand prints a human-readable message indicating that the
  CWD is not a managed bonsai worktree and exits with status 1

#### Scenario: Pool directory does not exist yet

- **WHEN** the user runs `bs current` before any slot has been provisioned (i.e.
  the pool directory does not exist)
- **THEN** the subcommand prints a human-readable message and exits with status
  1 without producing an error

### Requirement: Display branch name alongside slot path

The `current` subcommand SHALL append the branch name in parentheses after the
tilde-abbreviated path when the detected slot has a checked-out branch (i.e. is
not in detached HEAD state), using the format `<path> (<branch>)`. When the slot
is in detached HEAD state the subcommand MUST omit the branch suffix.

#### Scenario: Slot is on a named branch

- **WHEN** the user runs `bs current` and the slot has branch `my-feature`
  checked out
- **THEN** the output is `~/.bonsai/<repo>/<slot> (my-feature)`

#### Scenario: Slot is in detached HEAD state

- **WHEN** the user runs `bs current` and the slot is in detached HEAD state
- **THEN** the output is `~/.bonsai/<repo>/<slot>` with no branch suffix
