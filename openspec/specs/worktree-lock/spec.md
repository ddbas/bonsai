# worktree-lock Specification

## Purpose

TBD - created by archiving change worktree-lock-unlock. Update Purpose after
archive.

## Requirements

### Requirement: Lock a bonsai pool slot via git

The `bs lock` subcommand SHALL lock a bonsai-managed pool slot by delegating to
`git worktree lock`, preventing `bs get` from reusing that slot.

#### Scenario: Lock current slot with no arguments

- **WHEN** the user runs `bs lock` from inside a managed bonsai pool slot
- **THEN** the slot SHALL be locked via `git worktree lock <path>` and `bs get`
  SHALL no longer consider it available

#### Scenario: Lock slot by absolute path

- **WHEN** the user runs `bs lock /home/user/.bonsai/repo/a3f9c1b2` where that
  path is a bonsai-managed pool slot
- **THEN** the slot SHALL be locked via `git worktree lock`

### Requirement: Optional lock reason

The `bs lock` subcommand SHALL accept an optional `--reason <message>` flag
whose value is forwarded verbatim to `git worktree lock --reason`.

#### Scenario: Lock with reason string

- **WHEN** the user runs `bs lock --reason "reserved for agent build"` inside a
  managed slot
- **THEN** the slot SHALL be locked and git SHALL store the reason string in the
  worktree metadata

#### Scenario: Lock without reason

- **WHEN** the user runs `bs lock` without `--reason`
- **THEN** the slot SHALL be locked without a reason annotation (git default)

### Requirement: Reject non-pool targets

The `bs lock` subcommand SHALL error with a clear message when the given path is
not a bonsai-managed pool slot for the current repository.

#### Scenario: Path outside pool directory

- **WHEN** the user runs `bs lock /some/random/path`
- **THEN** `bs lock` SHALL exit with a non-zero status and print an error naming
  the pool directory

### Requirement: Default to current slot

The `bs lock` subcommand SHALL use the slot containing the current working
directory when no path argument is provided.

#### Scenario: No argument outside a managed slot

- **WHEN** the user runs `bs lock` from a directory that is not inside any
  bonsai pool slot
- **THEN** `bs lock` SHALL exit with a non-zero status and print an error
  indicating the CWD is not inside a managed slot
