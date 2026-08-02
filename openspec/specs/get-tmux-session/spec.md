## Purpose

Defines `bs get`'s optional tmux session integration: creating or reusing a tmux
session rooted at the provisioned worktree slot, deriving a default session name
from the repo/branch, accepting a custom session name, and controlling whether
the invoking terminal attaches to it.

## Requirements

### Requirement: `--tmux-session` opts into tmux session creation for `bs get`

`bs get --tmux-session` SHALL, after provisioning (or reusing) the worktree slot
as `bs get` does today, create a tmux session rooted at the slot's path if one
with the resolved name does not already exist. If a session with that name
already exists, it SHALL be reused (not recreated, not errored). Without
`--tmux-session`, `bs get` SHALL NOT invoke tmux or touch any tmux session,
preserving current default behavior exactly.

#### Scenario: `--tmux-session` creates a new session

- **WHEN** the user runs `bs get --tmux-session`
- **AND** no tmux session with the resolved name exists
- **THEN** the worktree slot SHALL be provisioned exactly as plain `bs get` does
- **THEN** a new detached tmux session SHALL be created with its working
  directory set to the provisioned slot path
- **THEN** the command SHALL exit with status 0

#### Scenario: `--tmux-session` reuses an existing session

- **WHEN** the user runs `bs get --tmux-session`
- **AND** a tmux session with the resolved name already exists
- **THEN** the worktree slot SHALL be provisioned exactly as plain `bs get` does
- **THEN** no new tmux session SHALL be created
- **THEN** the command SHALL exit with status 0

#### Scenario: Plain `bs get` never touches tmux

- **WHEN** the user runs `bs get` without `--tmux-session`
- **THEN** no tmux commands SHALL be invoked
- **THEN** behavior and output SHALL be identical to `bs get` before this change

#### Scenario: `tmux` is not installed

- **WHEN** the user runs `bs get --tmux-session`
- **AND** no `tmux` executable is found on `PATH`
- **THEN** the command SHALL exit with a non-zero status
- **THEN** stderr SHALL contain an actionable error message stating that `tmux`
  was not found

### Requirement: Default tmux session name follows the `worktree-get` naming convention

When `--tmux-session` is passed with no value, the session name SHALL be derived
as `🌳 <repo-name> (<branch-display>)`, where `<repo-name>` is the same
repository slug/name `bs get` already uses to locate the pool directory, and
`<branch-display>` is the branch name resolved via the positional `<branch>`
argument, `-b`, or `-B` when one was supplied, or the literal `detached` when no
branch flag/argument was supplied.

#### Scenario: Default name with an existing branch checked out

- **WHEN** the user runs `bs get --tmux-session my-feature`
- **AND** branch `my-feature` exists and is checked out via the positional
  argument
- **THEN** the tmux session name SHALL be `🌳 <repo-name> (my-feature)`

#### Scenario: Default name with `-b`/`-B`

- **WHEN** the user runs `bs get -b new-branch --tmux-session`
- **THEN** the tmux session name SHALL be `🌳 <repo-name> (new-branch)`

#### Scenario: Default name with no branch requested

- **WHEN** the user runs `bs get --tmux-session` with no positional branch,
  `-b`, or `-B`
- **THEN** the tmux session name SHALL be `🌳 <repo-name> (detached)`

### Requirement: `--tmux-session` accepts an optional custom name

`--tmux-session` SHALL accept an optional value. When a non-empty value is
supplied, that exact string SHALL be used as the tmux session name instead of
the derived default name.

#### Scenario: Custom session name is used verbatim

- **WHEN** the user runs `bs get --tmux-session=my-custom-session`
- **THEN** the tmux session created or reused SHALL be named exactly
  `my-custom-session`, not the derived default name

### Requirement: `--no-attach` suppresses attaching/switching to the session

`bs get --tmux-session` SHALL, by default (without `--no-attach`), attach the
invoking terminal to the created/reused session — using `switch-client`
semantics when already inside a tmux client, or `attach-session` semantics
otherwise. When `--no-attach` is also passed, the command SHALL create or reuse
the session without attaching or switching the invoking terminal to it, and
SHALL exit normally after printing its usual output.

#### Scenario: Default attaches to the session

- **WHEN** the user runs `bs get --tmux-session` without `--no-attach`
- **THEN** the invoking terminal SHALL be attached or switched to the
  created/reused tmux session

#### Scenario: `--no-attach` skips attaching

- **WHEN** the user runs `bs get --tmux-session --no-attach`
- **THEN** the session SHALL be created or reused
- **THEN** the invoking terminal SHALL NOT be attached or switched to it
- **THEN** the command SHALL exit with status 0 after returning control to the
  caller

### Requirement: `--no-attach` requires `--tmux-session`

`--no-attach` SHALL only be accepted when `--tmux-session` is also present on
the same invocation. Supplying `--no-attach` without `--tmux-session` SHALL
result in a non-zero exit and a usage error.

#### Scenario: `--no-attach` without `--tmux-session`

- **WHEN** the user runs `bs get --no-attach`
- **THEN** the CLI SHALL exit with a non-zero status
- **THEN** stderr SHALL indicate that `--no-attach` requires `--tmux-session`
