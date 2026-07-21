## Purpose

Provision a managed git worktree slot and check out an **existing** branch into
it in one step, so developers can grab a worktree for a branch that already
exists elsewhere in the repository (e.g. a teammate's feature branch) without
manually running `git checkout <branch>` inside the slot afterward.

## Requirements

### Requirement: `bs get <branch>` checks out an existing branch in the provisioned slot

`bs get <branch>` SHALL provision (or reuse) a worktree slot exactly as `bs get`
does today, then check out the **existing** branch `<branch>` inside that slot
via `git checkout <branch>` (no `-b`/`-B`). The branch SHALL NOT be created or
reset; it SHALL already exist in the repository.

#### Scenario: Positional branch argument checks out an existing branch

- **WHEN** the user runs `bs get my-existing-branch`
- **AND** a branch named `my-existing-branch` already exists in the repository
- **AND** it is not currently checked out in any other worktree
- **THEN** the slot SHALL be provisioned (reused or newly created)
- **THEN** `git checkout my-existing-branch` SHALL be run inside the slot
- **THEN** the slot SHALL be on branch `my-existing-branch` (not detached HEAD)
- **THEN** stdout SHALL contain `🌳` followed by the slot path and the branch
  name

### Requirement: Non-existent branch is rejected

`bs get <branch>` SHALL exit with a non-zero status and print an actionable
error message, and SHALL NOT leave a slot reset or a new worktree registered,
when the branch named in the positional argument does not exist anywhere in the
repository. This SHALL be enforced by the underlying `git checkout` /
`git worktree add` call failing naturally (git already refuses to check out a
non-existent branch); `bs get` SHALL NOT perform a separate existence check
before attempting the checkout.

#### Scenario: Branch does not exist

- **WHEN** the user runs `bs get no-such-branch`
- **AND** no branch named `no-such-branch` exists in the repository
- **THEN** the command SHALL exit with a non-zero status
- **THEN** stderr SHALL contain git's own error message naming `no-such-branch`
- **THEN** no worktree slot SHALL be created or reset

### Requirement: Branch already checked out elsewhere is rejected, unless it is a bonsai-managed slot for this repo

`bs get <branch>` SHALL exit with a non-zero status and print an actionable
error message identifying the conflicting worktree path, when the named branch
is already checked out in another worktree that is **not** a bonsai-managed pool
slot for this repository (whether unmanaged entirely, or a managed slot
belonging to a different repository's pool). This SHALL be enforced by the
underlying `git checkout` / `git worktree add` call failing naturally for those
cases; `bs get` SHALL NOT perform a separate already-checked-out check before
attempting the checkout in those cases.

When the named branch is already checked out in a bonsai-managed pool slot for
**this** repository, `bs get <branch>` SHALL NOT error, SHALL NOT invoke
`git checkout`/`git worktree add` for the branch, SHALL NOT reset or newly
register any slot, and SHALL instead print that existing slot's path (with the
branch name, consistent with the standard output format) and exit 0.

#### Scenario: Branch checked out in another managed slot for this repo

- **WHEN** the user runs `bs get shared-branch`
- **AND** `shared-branch` is currently checked out in another bonsai pool slot
  belonging to this repository
- **THEN** the command SHALL exit with status 0
- **THEN** stdout SHALL contain the path of the existing slot that already has
  `shared-branch` checked out, together with the branch name
- **THEN** no new slot SHALL be created and no existing slot SHALL be reset
- **THEN** `git checkout`/`git worktree add` SHALL NOT be invoked for
  `shared-branch`

#### Scenario: Branch checked out in a locked managed slot for this repo

- **WHEN** the user runs `bs get shared-branch`
- **AND** `shared-branch` is currently checked out in another bonsai pool slot
  for this repository that has been locked via `bs lock`
- **THEN** the command SHALL exit with status 0
- **THEN** stdout SHALL contain the path of that locked slot together with the
  branch name
- **THEN** the slot SHALL remain locked and untouched otherwise

#### Scenario: Branch checked out in an unmanaged worktree

- **WHEN** the user runs `bs get shared-branch`
- **AND** `shared-branch` is currently checked out in a worktree outside the
  bonsai pool (e.g. created manually via `git worktree add`)
- **THEN** the command SHALL exit with a non-zero status
- **THEN** stderr SHALL contain git's own error message identifying that
  worktree's path

### Requirement: Branch name is shown in output

The output line SHALL include the branch name in addition to the slot path,
consistent with the `-b`/`-B` output format, when `bs get` is invoked with the
positional `<branch>` argument.

#### Scenario: Branch name in stdout for positional argument

- **WHEN** the user runs `bs get my-existing-branch` successfully
- **THEN** stdout SHALL contain the branch name `my-existing-branch` alongside
  the slot path (e.g. `🌳 /path/to/slot  (my-existing-branch)`)
