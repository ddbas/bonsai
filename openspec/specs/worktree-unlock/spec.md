# worktree-unlock Specification

## Purpose

TBD - created by archiving change worktree-lock-unlock. Update Purpose after
archive.

## Requirements

### Requirement: Unlock a bonsai pool slot via git

The `bs unlock` subcommand SHALL unlock a bonsai-managed pool slot by delegating
to `git worktree unlock`, making the slot available for reuse by `bs get`.

#### Scenario: Unlock current slot with no arguments

- **WHEN** the user runs `bs unlock` from inside a managed bonsai pool slot that
  is locked
- **THEN** the slot SHALL be unlocked via `git worktree unlock <path>` and
  `bs get` SHALL consider it available (if otherwise clean)

#### Scenario: Unlock slot by absolute path

- **WHEN** the user runs `bs unlock /home/user/.bonsai/repo/a3f9c1b2` where that
  path is a locked bonsai pool slot
- **THEN** the slot SHALL be unlocked via `git worktree unlock`

### Requirement: Reject non-pool targets

The `bs unlock` subcommand SHALL error with a clear message when the given path
is not a bonsai-managed pool slot for the current repository.

#### Scenario: Path outside pool directory

- **WHEN** the user runs `bs unlock /some/random/path`
- **THEN** `bs unlock` SHALL exit with a non-zero status and print an error
  naming the pool directory

### Requirement: Default to current slot

The `bs unlock` subcommand SHALL use the slot containing the current working
directory when no path argument is provided.

#### Scenario: No argument outside a managed slot

- **WHEN** the user runs `bs unlock` from a directory that is not inside any
  bonsai pool slot
- **THEN** `bs unlock` SHALL exit with a non-zero status and print an error
  indicating the CWD is not inside a managed slot

### Requirement: Error on already-unlocked slot

The `bs unlock` subcommand SHALL surface git's error unchanged when attempting
to unlock a slot that is not currently locked.

#### Scenario: Unlock a slot that is not locked

- **WHEN** the user runs `bs unlock` on a slot that has no git lock
- **THEN** `bs unlock` SHALL exit with a non-zero status and propagate git's
  error message
