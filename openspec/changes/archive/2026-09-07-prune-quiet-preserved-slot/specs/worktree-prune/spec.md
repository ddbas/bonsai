## REMOVED Requirements

### Requirement: Preserved slot is reported distinctly from deleted slots

## MODIFIED Requirements

### Requirement: At least one available slot is always preserved

`bs prune` SHALL NOT delete every available slot in the pool. When one or more
slots are classified `available`, `bs prune` SHALL select exactly one of them to
preserve — the available slot that appears first in the same pool ordering used
by `bs list`/`bs status` — and SHALL exclude it from deletion. All other
available slots SHALL be deleted as usual. `bs prune` SHALL NOT print any output
identifying the preserved slot; a successful preserve is silent.

#### Scenario: Multiple available slots, one preserved

- **WHEN** the user runs `bs prune` against a pool containing three available
  slots
- **THEN** exactly one of the three available slots' directories remains on disk
  and registered after the run, and the other two are deleted, and stdout
  contains no line identifying which slot was preserved

#### Scenario: Single available slot preserved instead of deleted

- **WHEN** the user runs `bs prune` against a pool containing exactly one
  available slot
- **THEN** that slot's directory is not deleted and remains registered as an
  available worktree after the run, and stdout reports that there was nothing to
  prune (identical to the output when there were zero available slots)

### Requirement: Preserved slot branch is detached

`bs prune` SHALL detach the preserved available slot's branch (reset it to
detached HEAD at the current repository HEAD commit) if that slot has a branch
checked out, so it remains immediately reusable and consistent with the
invariant that available slots carry no branch state forward. If the preserved
slot is already in detached HEAD, `bs prune` SHALL leave it untouched. This
detach, when successful, SHALL produce no stdout output of its own.

#### Scenario: Preserved slot had a branch checked out

- **WHEN** `bs prune` preserves an available slot that has branch `my-feature`
  checked out
- **THEN** after `bs prune` completes, that slot is in detached HEAD state,
  `my-feature` is no longer checked out in that slot, and stdout contains no
  line mentioning `my-feature` or the preserved slot

#### Scenario: Preserved slot already detached

- **WHEN** `bs prune` preserves an available slot that is already in detached
  HEAD state
- **THEN** `bs prune` performs no checkout operation on that slot and prints no
  output about it

### Requirement: Failure to detach the preserved slot does not delete it or abort the run

If detaching the preserved slot's branch fails, `bs prune` SHALL NOT delete that
slot's directory, SHALL report the detach failure to the user, SHALL still
attempt deletion of the other available slots and run `git worktree prune`, and
SHALL exit with a non-zero status.

#### Scenario: Detach of preserved slot fails

- **WHEN** `bs prune` attempts to detach the branch of the preserved available
  slot and the underlying `git checkout --detach` fails
- **THEN** the preserved slot's directory is not deleted, the failure is
  reported to the user, the other available slots are still processed for
  deletion, `git worktree prune` still runs, and the process exits with a
  non-zero status

### Requirement: Empty or non-existent pool is handled gracefully

`bs prune` SHALL print a friendly message indicating there is nothing to prune
and SHALL exit with code 0, without attempting to delete anything, when no pool
directory exists yet for the current repository, or the pool exists but contains
no slots. `bs prune` SHALL print this same "nothing to prune" message, and exit
with code 0, when the only outcome of the run was a successful preserve (i.e. no
slots were deleted and no failures occurred), so a run that only preserved a
slot is indistinguishable from a run where there was nothing to do at all.

#### Scenario: Pool directory does not exist

- **WHEN** the user runs `bs prune` in a repository that has never had `bs get`
  run
- **THEN** the process exits with code 0 and prints a message indicating no
  worktrees are managed for this repository

#### Scenario: Only a successful preserve occurred

- **WHEN** the user runs `bs prune` against a pool containing exactly one
  available slot and no other slots
- **THEN** the process exits with code 0 and prints the same "nothing to prune"
  message as when there were zero available slots
