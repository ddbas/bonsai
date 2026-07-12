## ADDED Requirements

### Requirement: `-b` flag creates a new branch in the provisioned slot

`bs get -b <branch>` SHALL provision (or reuse) a worktree slot exactly as
`bs get` does today, then check out a **new** branch named `<branch>` at the
resolved HEAD inside that slot. If the branch already exists in the repository
the command SHALL exit with a non-zero status and an actionable error message.
This mirrors `git checkout -b` semantics.

#### Scenario: `-b` with a fresh branch name

- **WHEN** the user runs `bs get -b my-feature`
- **AND** no branch named `my-feature` exists in the repository
- **THEN** the slot SHALL be provisioned (reused or newly created)
- **THEN** `git checkout -b my-feature` SHALL be run inside the slot
- **THEN** the slot SHALL be on branch `my-feature` (not detached HEAD)
- **THEN** stdout SHALL contain `🌳` followed by the slot path

#### Scenario: `-b` with an already-existing branch name

- **WHEN** the user runs `bs get -b existing-branch`
- **AND** a branch named `existing-branch` already exists in the repository
- **THEN** the command SHALL exit with a non-zero status
- **THEN** stderr SHALL contain an actionable error message that names the
  conflicting branch

#### Scenario: `-b` with no branch name argument

- **WHEN** the user runs `bs get -b` with no following argument
- **THEN** the CLI SHALL exit with a non-zero status and print usage help

### Requirement: `-B` flag creates or resets a branch in the provisioned slot

`bs get -B <branch>` SHALL provision (or reuse) a worktree slot exactly as
`bs get` does today, then check out branch `<branch>` at the resolved HEAD
inside that slot, creating it if it does not exist and **resetting** it to HEAD
if it does. This mirrors `git checkout -B` semantics. The command SHALL NOT fail
when the branch already exists.

#### Scenario: `-B` creates branch when it does not exist

- **WHEN** the user runs `bs get -B new-branch`
- **AND** no branch named `new-branch` exists
- **THEN** the slot SHALL be provisioned
- **THEN** `git checkout -B new-branch` SHALL be run inside the slot
- **THEN** the slot SHALL be on branch `new-branch` at the resolved HEAD

#### Scenario: `-B` resets branch when it already exists

- **WHEN** the user runs `bs get -B existing-branch`
- **AND** a branch named `existing-branch` already exists at some other commit
- **THEN** the slot SHALL be provisioned
- **THEN** `git checkout -B existing-branch` SHALL be run inside the slot
- **THEN** the branch SHALL be reset to the resolved HEAD inside the slot
- **THEN** the command SHALL exit with code 0

#### Scenario: `-B` with no branch name argument

- **WHEN** the user runs `bs get -B` with no following argument
- **THEN** the CLI SHALL exit with a non-zero status and print usage help

### Requirement: `-b` and `-B` are mutually exclusive

The `-b` and `-B` flags SHALL NOT be accepted together. Supplying both on the
same invocation SHALL result in a non-zero exit and a usage error.

#### Scenario: Both flags supplied

- **WHEN** the user runs `bs get -b foo -B bar`
- **THEN** the CLI SHALL exit with a non-zero status
- **THEN** stderr SHALL describe the mutual-exclusion constraint

### Requirement: Branch name is shown in output

When `bs get` is invoked with `-b` or `-B`, the output line SHALL include the
branch name in addition to the slot path.

#### Scenario: Branch name in stdout

- **WHEN** the user runs `bs get -b my-feature` successfully
- **THEN** stdout SHALL contain the branch name `my-feature` alongside the slot
  path (e.g. `🌳 /path/to/slot  (my-feature)`)
