## ADDED Requirements

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

### Requirement: Report includes an overall classification using the same priority rules `bs list` used

The report SHALL classify the slot as one of `locked`, `in use`, or `available`,
using the same priority order previously used by `bs list`:

1. **`locked`** — if the slot is git-locked, regardless of other signals.
2. **`in use`** — if the slot is not locked but has uncommitted changes,
   untracked files, or at least one process with an open file descriptor
   directly in the slot root.
3. **`available`** — if the slot is not locked, its working tree is clean, and
   no process has an open handle directly at the slot root.

#### Scenario: Locked slot classified as locked regardless of dirty state

- **WHEN** the resolved slot is git-locked and also has uncommitted changes and
  open processes
- **THEN** the report SHALL classify the slot as `locked`

#### Scenario: Dirty unlocked slot classified as in use

- **WHEN** the resolved slot is unlocked and has uncommitted changes
- **THEN** the report SHALL classify the slot as `in use`

#### Scenario: Clean idle unlocked slot classified as available

- **WHEN** the resolved slot is unlocked, clean, and has no open processes at
  its root
- **THEN** the report SHALL classify the slot as `available`

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

### Requirement: Report lists individual open processes with PID and command name

The report SHALL list each distinct process with an open file descriptor
directly in the slot root (non-recursive, same detection as
`count_open_processes`), showing its PID and command name — not merely a count.

#### Scenario: Two distinct processes have handles open

- **WHEN** `lsof -w +d <slot>` reports two distinct PIDs with open handles
  directly in the slot root
- **THEN** the report SHALL list both processes, each with its PID and command
  name

#### Scenario: No processes have handles open

- **WHEN** no process has an open file descriptor directly in the slot root
- **THEN** the report SHALL indicate there are no open processes (e.g. by
  omitting the section or stating none)

#### Scenario: Process has handles only in a subdirectory

- **WHEN** a process has an open file descriptor only in a subdirectory of the
  slot (e.g. `<slot>/src/main.rs`) and not at the slot root itself
- **THEN** that process SHALL NOT appear in the report's process list

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

### Requirement: lsof unavailability is a hard error

`bs status` SHALL exit with a non-zero status and an actionable error message
naming `lsof` as the missing dependency when `lsof` cannot be spawned, rather
than silently omitting process information, consistent with existing
`has_open_files`/`count_open_processes` behavior.

#### Scenario: `lsof` not on PATH

- **WHEN** `lsof` is not present on `PATH`
- **THEN** `bs status` SHALL exit with a non-zero status
- **THEN** stderr SHALL contain a message naming `lsof` as the missing
  dependency
