# worktree-prune Specification

## Purpose

TBD - created by archiving change worktree-prune. Update Purpose after archive.

## Requirements

### Requirement: `prune` subcommand exists

The CLI SHALL expose a `prune` subcommand (`bs prune`) that reclaims disk space
used by unused bonsai pool slots for the current repository, by deleting the
directories of slots classified `available` and then delegating deregistration
to `git worktree prune`.

#### Scenario: Invoke with `prune`

- **WHEN** the user runs `bs prune`
- **THEN** the process exits with code 0 (assuming no deletion errors) and
  prints a summary of what was pruned to stdout

### Requirement: Only slots classified `available` are deleted

`bs prune` SHALL classify every managed pool slot for the current repository
using the same priority rules already used by `bs list`/`bs status` (`locked` >
`in use` > `available`). Only slots classified `available` SHALL have their
on-disk directory deleted. Slots classified `locked` or `in use` SHALL NOT be
deleted or otherwise modified.

#### Scenario: Mixed pool with locked, in-use, and available slots

- **WHEN** the user runs `bs prune` against a pool containing one locked slot,
  one in-use (dirty or open-file) slot, and one available slot
- **THEN** only the available slot's directory is deleted; the locked and in-use
  slots' directories remain untouched on disk

#### Scenario: No available slots

- **WHEN** the user runs `bs prune` against a pool where every slot is either
  locked or in use
- **THEN** no directories are deleted, `git worktree prune` is still run, and
  the process exits with code 0 printing a message indicating there was nothing
  to prune

### Requirement: `git worktree prune` performs the deregistration

`bs prune` SHALL NOT itself deregister worktrees, delete branches, or edit git's
internal worktree administrative files. After deleting the on-disk directories
of `available` slots, `bs prune` SHALL invoke `git worktree prune` exactly once
so git deregisters the worktree entries whose directories no longer exist.

#### Scenario: Available slot directory deleted then deregistered

- **WHEN** `bs prune` deletes the directory of an available slot
- **THEN** `bs prune` subsequently runs `git worktree prune`, after which
  `git worktree list --porcelain` no longer reports that slot as a registered
  worktree

#### Scenario: `git worktree prune` runs even when nothing was deleted

- **WHEN** `bs prune` is run and no slot qualifies for deletion (e.g. no pool
  exists yet, or all slots are locked/in use)
- **THEN** `bs prune` still invokes `git worktree prune` before exiting
  successfully

### Requirement: Pruned slots are reported with path and branch

For every slot whose directory `bs prune` deletes, `bs prune` SHALL print that
slot's identifying information: its tilde-abbreviated path, and, if the slot had
a branch checked out (not detached HEAD), the branch name in parentheses —
matching the formatting already used by `bs list`/`bs get`.

#### Scenario: Pruned slot with a checked-out branch

- **WHEN** `bs prune` deletes an available slot that had branch `my-feature`
  checked out
- **THEN** stdout includes a line containing the slot's tilde-abbreviated path
  and `(my-feature)`

#### Scenario: Pruned slot in detached HEAD

- **WHEN** `bs prune` deletes an available slot that was in detached HEAD state
  (no branch)
- **THEN** stdout includes a line containing the slot's tilde-abbreviated path
  with no branch suffix

### Requirement: Empty or non-existent pool is handled gracefully

`bs prune` SHALL print a friendly message indicating there is nothing to prune
and SHALL exit with code 0, without attempting to delete anything, when no pool
directory exists yet for the current repository, or the pool exists but contains
no slots.

#### Scenario: Pool directory does not exist

- **WHEN** the user runs `bs prune` in a repository that has never had `bs get`
  run
- **THEN** the process exits with code 0 and prints a message indicating no
  worktrees are managed for this repository

### Requirement: Per-slot deletion failures do not abort the whole run

`bs prune` SHALL report the failure for that slot, continue attempting to delete
the remaining available slots, and still invoke `git worktree prune` at the end,
if deleting one available slot's directory fails (e.g. a filesystem error).
`bs prune` SHALL exit with a non-zero status if at least one slot's directory
failed to delete, and with status 0 if every attempted deletion succeeded
(including the case of zero attempted deletions).

#### Scenario: One of several available slots fails to delete

- **WHEN** `bs prune` attempts to delete two available slots' directories and
  one deletion fails while the other succeeds
- **THEN** the successfully deleted slot is reported as pruned, the failure for
  the other slot is reported to the user, `git worktree prune` still runs, and
  the process exits with a non-zero status
