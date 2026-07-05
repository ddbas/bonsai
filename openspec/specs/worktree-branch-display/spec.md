## Purpose

Display the checked-out branch name alongside each worktree path in `bs list`,
giving users immediate visibility into which branch each pool slot is on without
having to inspect each directory manually.

## Requirements

### Requirement: Branch name is shown after the path in bold parentheses

`bs list` SHALL display the checked-out branch name immediately after the
tilde-abbreviated path, enclosed in parentheses and rendered in bold, for every
pool worktree that has a branch checked out (i.e. is not in detached-HEAD
state).

#### Scenario: Worktree on a named branch

- **WHEN** a pool worktree has branch `feature/my-work` checked out
- **THEN** `bs list` SHALL include `(feature/my-work)` in bold immediately after
  the path on that line

#### Scenario: Worktree in detached HEAD state

- **WHEN** a pool worktree is in detached HEAD state (no branch checked out)
- **THEN** `bs list` SHALL NOT append any parenthesised branch suffix to the
  path for that line

#### Scenario: Branch data comes from worktree list porcelain

- **WHEN** `git worktree list --porcelain` emits `branch refs/heads/main` for a
  slot
- **THEN** the branch name displayed SHALL be `main` (the short name, with
  `refs/heads/` stripped)
