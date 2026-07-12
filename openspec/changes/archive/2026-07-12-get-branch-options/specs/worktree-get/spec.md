## MODIFIED Requirements

### Requirement: `get` subcommand exists and is the default

The CLI SHALL expose a `get` subcommand. Running `bs get` SHALL NOT require any
arguments to succeed. Running `bs` with no subcommand SHALL behave identically
to `bs get`. The subcommand MAY accept optional `-b <branch>` or `-B <branch>`
flags; their absence leaves all existing behaviour unchanged.

#### Scenario: Invoke with explicit subcommand

- **WHEN** the user runs `bs get`
- **THEN** the process exits with code 0 and prints the worktree path to stdout

#### Scenario: Invoke without any subcommand

- **WHEN** the user runs `bs` with no subcommand and no arguments
- **THEN** the process behaves as if `bs get` was called

#### Scenario: Invoke with `-b` flag

- **WHEN** the user runs `bs get -b <branch>`
- **THEN** the process provisions a slot and creates branch `<branch>` in it

#### Scenario: Invoke with `-B` flag

- **WHEN** the user runs `bs get -B <branch>`
- **THEN** the process provisions a slot and creates or resets branch `<branch>`
  in it
