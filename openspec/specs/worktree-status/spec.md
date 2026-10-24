## Purpose

Provide a `bs status [PATH]` subcommand that reports detailed, itemized status
for a single bonsai-managed pool worktree slot — lock state and reason,
uncommitted/untracked files, and open processes — complementing `bs list`'s
at-a-glance badge with per-slot detail.

## Requirements

### Requirement: `status` subcommand exists

The CLI SHALL expose a `status` subcommand accepting an optional positional
`PATH` argument: `bs status [PATH]`.

#### Scenario: Invoke with explicit path

- **WHEN** the user runs `bs status <path>` where `<path>` is a valid
  bonsai-managed pool slot for the current repository
- **THEN** the process exits with code 0 and prints the detailed status report
  for that slot to stdout

#### Scenario: Invoke with no argument inside a managed slot

- **WHEN** the user runs `bs status` from inside a managed bonsai pool slot (or
  a subdirectory of one)
- **THEN** the process exits with code 0 and prints the detailed status report
  for the containing slot

### Requirement: Defaults to the current bonsai worktree when no path is given

When `PATH` is omitted, `bs status` SHALL resolve the target slot using the same
current-working-directory detection `bs current` uses. If the current working
directory is not inside any managed pool slot for this repository, `bs status`
SHALL exit with a non-zero status and print an actionable error message
instructing the user to provide a path argument.

#### Scenario: CWD is inside a managed slot

- **WHEN** the user runs `bs status` with no arguments from
  `~/.bonsai/repo/a3f9c1b2/src` (a subdirectory of a managed slot)
- **THEN** `bs status` SHALL report on the `~/.bonsai/repo/a3f9c1b2` slot

#### Scenario: CWD is not inside any managed slot

- **WHEN** the user runs `bs status` with no arguments from a directory that is
  not inside any managed pool slot
- **THEN** the process SHALL exit with a non-zero status
- **THEN** stderr SHALL contain a message indicating the CWD is not inside a
  managed bonsai pool slot and that a path argument is required

### Requirement: Explicit path must be a bonsai-managed pool slot for this repository

When `PATH` is given, `bs status` SHALL validate it the same way `bs lock` and
`bs unlock` validate their target path: the path must exist on disk and must
resolve (after canonicalization) to a location under this repository's pool
directory.

#### Scenario: Path does not exist

- **WHEN** the user runs `bs status <path>` where `<path>` does not exist on
  disk
- **THEN** the process SHALL exit with a non-zero status and print an error
  naming the missing path

#### Scenario: Path exists but is outside the pool

- **WHEN** the user runs `bs status <path>` where `<path>` exists but is not
  under this repository's managed pool directory
- **THEN** the process SHALL exit with a non-zero status and print an error
  stating the path is not a bonsai-managed pool slot

### Requirement: Report shows the resolved slot's path and branch

The report SHALL include the tilde-abbreviated path of the resolved slot and,
when applicable, the checked-out branch name (omitted for detached HEAD).

#### Scenario: Slot has a branch checked out

- **WHEN** the resolved slot has branch `feature-x` checked out
- **THEN** the report SHALL include the tilde-abbreviated path followed by
  `feature-x` in parentheses

#### Scenario: Slot is in detached HEAD

- **WHEN** the resolved slot is in detached HEAD state
- **THEN** the report SHALL include the tilde-abbreviated path with no branch
  annotation

### Requirement: Report includes the lock reason when the slot is locked

When the slot is git-locked, the report SHALL include the lock's `--reason` text
if one was set, or indicate that no reason was given.

#### Scenario: Locked with a reason

- **WHEN** the resolved slot was locked with
  `git worktree lock --reason "build in progress"`
- **THEN** the report SHALL display `build in progress` as the lock reason

#### Scenario: Locked with no reason

- **WHEN** the resolved slot is git-locked without a `--reason` argument
- **THEN** the report SHALL indicate the slot is locked without a reason string

### Requirement: Report lists individual uncommitted and untracked files

The report SHALL list the individual `git status --porcelain` lines for the
slot, split into uncommitted (modified/staged, non-`??` XY code) and untracked
(`??` XY code) sections — not merely counts.

#### Scenario: Slot has modified and untracked files

- **WHEN** `git status --porcelain` for the slot reports one modified file and
  one untracked file
- **THEN** the report SHALL list the modified file under an uncommitted section
  and the untracked file under an untracked section

#### Scenario: Slot is clean

- **WHEN** `git status --porcelain` for the slot reports no output
- **THEN** the report SHALL indicate there are no uncommitted or untracked files
  (e.g. by omitting the sections or stating none)

### Requirement: Report classification uses lock, dirty, and branch state with the same priority as `bs list`

The report SHALL classify the slot as one of `locked`, `in use`, or `available`,
using the same priority order `bs list`'s status badge uses - both are computed
by one shared classification function so the two commands cannot disagree on a
given slot:

1. **`locked`** - if the slot is git-locked, regardless of other signals.
2. **`in use`** - if the slot is not locked but has uncommitted changes,
   untracked files, or a branch state carrying unmerged work (see
   `worktree-branch-state`).
3. **`available`** - if the slot is not locked, its working tree is clean, and
   its branch carries no unmerged work.

Open processes SHALL NOT affect the classification.

#### Scenario: Locked slot classified as locked regardless of dirty state

- **WHEN** the resolved slot is git-locked and also has uncommitted changes and
  unmerged commits
- **THEN** the report SHALL classify the slot as `locked`

#### Scenario: Dirty unlocked slot classified as in use

- **WHEN** the resolved slot is unlocked and has uncommitted changes
- **THEN** the report SHALL classify the slot as `in use`

#### Scenario: Clean slot with unmerged work classified as in use

- **WHEN** the resolved slot is unlocked and clean but its branch has commits
  not on the default ref and not proven merged
- **THEN** the report SHALL classify the slot as `in use`

#### Scenario: Clean slot with no unmerged work classified as available

- **WHEN** the resolved slot is unlocked, clean, and its branch carries no
  unmerged work
- **THEN** the report SHALL classify the slot as `available`

### Requirement: Report includes a branch-state line explaining the classification

The report SHALL include a "Branch state" line stating why the branch is or is
not considered to carry unmerged work, using the reason defined in
`worktree-branch-state` (e.g. "3 commits ahead of origin/main, no upstream
configured", "remote branch still exists", "default branch could not be
determined", "no commits of its own"). When the reason is that commits were
never pushed with upstream tracking, the line SHALL hint that pushing with `-u`
enables merged-branch detection.

#### Scenario: Unmerged commits, never pushed

- **WHEN** the resolved slot's branch has 3 commits not on `origin/main` and no
  upstream config
- **THEN** the report SHALL show a branch-state line stating 3 commits ahead of
  `origin/main` and that no upstream is configured

#### Scenario: Merged and remote gone

- **WHEN** the resolved slot's branch was squash-merged and its remote branch is
  gone
- **THEN** the report SHALL show a branch-state line stating the branch is
  merged and its remote branch is gone
