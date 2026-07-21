## MODIFIED Requirements

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
