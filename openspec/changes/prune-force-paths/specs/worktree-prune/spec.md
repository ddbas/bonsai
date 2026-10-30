## MODIFIED Requirements

### Requirement: Only slots classified `available` are deleted

`bs prune` SHALL classify every managed pool slot for the current repository
using the same priority rules already used by `bs list`/`bs status` (`locked` >
`in use` > `available`). Without `--force`, only slots classified `available`
SHALL be considered for deletion, and slots classified `locked` or `in use`
SHALL NOT be deleted or otherwise modified. With `--force`, `in use` candidate
slots are also eligible, as specified by the "`--force` makes `in use` slots
prunable" requirement; `locked` slots SHALL NEVER be deleted or modified. Among
the eligible slots, `bs prune` SHALL preserve exactly one (per the "At least one
available slot is always preserved" requirement) and delete the on-disk
directories of all the others.

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

## ADDED Requirements

### Requirement: `prune` accepts optional worktree paths

`bs prune` SHALL accept zero or more optional positional worktree path
arguments. When one or more paths are given, only the slots at those paths are
_candidates_ for deletion or sanitizing; all other slots SHALL NOT be deleted,
reset, or otherwise modified (except that an untargeted `available` slot MAY be
the preserved slot and is then left untouched). When no paths are given, every
pool slot is a candidate. Relative paths SHALL be resolved against the current
working directory. All other requirements in this spec (preservation,
`git worktree prune`, reporting, failure handling) apply to the candidate set.

#### Scenario: Prune a single available slot by path

- **WHEN** the user runs `bs prune <path>` where `<path>` is an `available` slot
  and another `available` slot exists in the pool
- **THEN** only the slot at `<path>` is deleted and reported as pruned, the
  other slot is preserved untouched, `git worktree prune` runs, and the process
  exits with code 0

#### Scenario: Targeted slot is the only available slot

- **WHEN** the user runs `bs prune <path>` where `<path>` is the only
  `available` slot in the pool
- **THEN** that slot is preserved (detached if it had a branch), nothing is
  deleted, and the output is the "nothing to prune" message

#### Scenario: Path is not a managed slot

- **WHEN** the user runs `bs prune <path>` where `<path>` is not a bonsai pool
  slot of the current repository
- **THEN** the process exits non-zero with an error naming the path, and
  nothing is deleted or modified

#### Scenario: In-use slot targeted without force

- **WHEN** the user runs `bs prune <path>` where `<path>` is an `in use` slot
- **THEN** the process exits non-zero with an error that `--force` is required,
  and nothing is deleted or modified

### Requirement: `--force` makes `in use` slots prunable

`bs prune` SHALL accept a `--force` option, with or without path arguments. With
`--force`, candidate slots classified `in use` SHALL be eligible for pruning
like `available` slots: those not chosen for preservation SHALL have their
directories deleted and be reported as pruned, even if they have uncommitted
changes, untracked files, or an unmerged branch. `--force` SHALL NOT delete any
git branch. Without paths, `--force` applies to every `in use` slot in the pool.

#### Scenario: Force-prune an in-use slot by path

- **WHEN** the user runs `bs prune --force <path>` where `<path>` is a dirty
  `in use` slot and another `available` slot exists
- **THEN** the slot at `<path>` is deleted and reported as pruned, `git worktree
  prune` runs, and the process exits with code 0

#### Scenario: Force with no paths

- **WHEN** the user runs `bs prune --force` against a pool with one locked slot,
  two `in use` slots, and one `available` slot
- **THEN** the two `in use` slots are deleted, the `available` slot is
  preserved, the locked slot is untouched, and the process exits with code 0

#### Scenario: Force does not delete branches

- **WHEN** `bs prune --force` deletes a slot that had branch `my-feature`
  checked out
- **THEN** branch `my-feature` still exists in the repository afterwards

### Requirement: Forced preserved slot is sanitized to `available`

When `--force` is passed and the slot chosen for preservation is classified `in
use`, `bs prune` SHALL sanitize it so that it classifies as `available`: reset
HEAD to the repository's current HEAD commit in detached HEAD state (detaching
any checked-out branch without deleting it), discard all uncommitted changes
(staged and unstaged, including in-progress merge state), and remove all
untracked files and directories. Git-ignored files SHALL NOT be removed. If
sanitizing fails, the slot SHALL NOT be deleted, the failure SHALL be reported,
the remaining candidates SHALL still be processed, `git worktree prune` SHALL
still run, and the process SHALL exit non-zero.

#### Scenario: Preserved dirty slot is cleaned

- **WHEN** `bs prune --force` preserves a slot that has branch `my-feature`
  checked out, a modified tracked file, and an untracked file
- **THEN** afterwards the slot is in detached HEAD at the repository HEAD
  commit, has a clean `git status`, `bs status` reports it `available`, and
  `my-feature` still exists

#### Scenario: Only in-use slots exist

- **WHEN** the user runs `bs prune --force` against a pool where every
  non-locked slot is `in use`
- **THEN** exactly one of them is sanitized and preserved as `available` and the
  others are deleted

#### Scenario: Sanitizing fails

- **WHEN** sanitizing the preserved `in use` slot fails
- **THEN** that slot's directory is not deleted, the failure is reported, other
  candidates are still processed, `git worktree prune` runs, and the exit code
  is non-zero

### Requirement: Preserved slot selection with candidates and force

The preserved slot SHALL be chosen in this order: (1) an `available` slot that
is not a candidate, if one exists (it is left completely untouched); otherwise
(2) the first `available` candidate in pool order; otherwise (3) with `--force`,
the first `in use` candidate in pool order (which is then sanitized). If none
qualifies, nothing is preserved. The "At least one available slot is always
preserved" and "Preserved slot branch is detached" requirements continue to
apply to a chosen candidate.

#### Scenario: Untargeted available slot satisfies preservation

- **WHEN** the user runs `bs prune --force <in-use-path>` and the pool contains
  an untargeted `available` slot
- **THEN** the untargeted slot is left untouched and the targeted slot is
  deleted

#### Scenario: Prefer an already-available candidate

- **WHEN** the user runs `bs prune --force` against a pool with an `in use` slot
  listed first and an `available` slot listed second
- **THEN** the `available` slot is preserved and the `in use` slot is deleted
  (not sanitized)

### Requirement: Locked slots can never be pruned

`bs prune` SHALL NEVER delete, reset, clean, or otherwise modify a slot
classified `locked`, regardless of `--force`. Explicitly targeting a locked slot
by path SHALL be an error whose message states the slot is locked and must be
unlocked (`bs unlock`) before it can be pruned. Locked slots that are not
explicitly targeted SHALL be skipped silently. A locked slot never counts as
the preserved slot.

#### Scenario: Locked slot targeted without force

- **WHEN** the user runs `bs prune <path>` where `<path>` is a locked slot
- **THEN** the process exits non-zero with an error saying the slot is locked
  and must be unlocked first, and nothing is deleted or modified

#### Scenario: Locked slot targeted with force

- **WHEN** the user runs `bs prune --force <path>` where `<path>` is a locked
  slot
- **THEN** the process exits non-zero with the same locked error, and nothing
  is deleted or modified

#### Scenario: Force without paths skips locked slots

- **WHEN** the user runs `bs prune --force` and the pool contains a locked,
  dirty slot
- **THEN** that slot is left exactly as it was (directory, uncommitted changes,
  branch) and the process does not fail because of it

#### Scenario: Prune after unlocking

- **WHEN** the user runs `bs unlock <path>` and then `bs prune --force <path>`
  on a slot that was locked and in use
- **THEN** the slot is eligible and is pruned or sanitized per the rules above

### Requirement: Targets are validated before any change

When paths are given, `bs prune` SHALL validate every target before deleting or
modifying anything. If any target is not a managed slot, is `locked`, or is
`in use` without `--force`, `bs prune` SHALL report every offending target,
change nothing, and exit non-zero. Duplicate paths SHALL be treated as one.

#### Scenario: One invalid target among valid ones

- **WHEN** the user runs `bs prune --force <available> <locked>`
- **THEN** the locked error is reported, neither slot is deleted or modified,
  and the process exits non-zero

#### Scenario: Duplicate paths

- **WHEN** the same slot path is passed more than once
- **THEN** the slot is processed once
