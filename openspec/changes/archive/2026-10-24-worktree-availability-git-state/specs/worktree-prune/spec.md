## MODIFIED Requirements

### Requirement: Only slots classified `available` are deleted

`bs prune` SHALL classify every managed pool slot for the current repository
using the same priority rules already used by `bs list`/`bs status` (`locked` >
`in use` > `available`). Only slots classified `available` SHALL be considered
for deletion, and slots classified `locked` or `in use` SHALL NOT be deleted or
otherwise modified. Among the slots classified `available`, `bs prune` SHALL
preserve exactly one (per the "At least one available slot is always preserved"
requirement) and delete the on-disk directories of all the others.

#### Scenario: Mixed pool with locked, in-use, and multiple available slots

- **WHEN** the user runs `bs prune` against a pool containing one locked slot,
  one in-use (dirty or unmerged-branch) slot, and two available slots
- **THEN** the locked and in-use slots' directories remain untouched on disk,
  one of the two available slots' directories is deleted, and the other
  available slot's directory is preserved (not deleted)

#### Scenario: No available slots

- **WHEN** the user runs `bs prune` against a pool where every slot is either
  locked or in use
- **THEN** no directories are deleted, `git worktree prune` is still run, and
  the process exits with code 0 printing a message indicating there was nothing
  to prune

#### Scenario: Exactly one available slot

- **WHEN** the user runs `bs prune` against a pool containing exactly one
  available slot (and any number of locked/in-use slots)
- **THEN** that available slot's directory is not deleted; it is preserved (and
  detached per the requirement below if it had a branch checked out)
