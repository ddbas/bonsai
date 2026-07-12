## MODIFIED Requirements

### Requirement: Each worktree is shown on its own line with path, branch, status, and usage stats

`bs list` SHALL print one line per managed pool worktree. Each line SHALL
contain:

1. A coloured status badge:
   - `available` in **green** — clean, unlocked, idle slot.
   - `in use` in **red** — slot has open processes, uncommitted changes, or
     other activity, and is not locked.
   - `locked` in **yellow** — slot is git-locked (regardless of whether it also
     has open processes or uncommitted changes; `locked` takes priority over
     `in use`).
2. The worktree path (with home directory prefix replaced with `~`).
3. Optionally, the checked-out branch name in **bold parentheses** immediately
   after the path (omitted for detached HEAD).
4. A compact usage-stats column that shows only non-zero values for: open
   processes (`⚙N`), uncommitted files (`±N`), and untracked files (`?N`),
   space-separated. The column is blank for clean idle slots. The stats column
   SHALL appear on `locked` rows when values are non-zero (a locked slot may
   still have open processes or uncommitted work).
5. When the slot is the one that contains the process's current working
   directory, the line SHALL be prefixed with `▶` so that the active slot is
   visually distinct from the rest. All other lines SHALL retain their existing
   format with a two-space indent in place of the arrow.

#### Scenario: Single available worktree, detached HEAD

- **WHEN** the pool contains one slot that is clean, unlocked, has no open file
  handles, and is in detached HEAD state
- **THEN** stdout SHALL contain one line with a green `available` badge, the
  tilde-prefixed path, no branch suffix, and a blank stats column

#### Scenario: Single available worktree with a branch

- **WHEN** the pool contains one clean, unlocked, idle slot with branch `main`
  checked out
- **THEN** stdout SHALL contain one line with a green `available` badge, the
  tilde-prefixed path followed by `(main)` in bold, and a blank stats column

#### Scenario: In-use worktree with open file handles

- **WHEN** a slot has 2 open processes and is not locked
- **THEN** the badge SHALL be red `in use` and the stats column SHALL contain
  `⚙2`

#### Scenario: In-use worktree with uncommitted changes only

- **WHEN** a slot has 3 modified/staged files, no open processes, no untracked
  files, and is not locked
- **THEN** the badge SHALL be red `in use` and the stats column SHALL contain
  `±3`

#### Scenario: In-use worktree with all three stats

- **WHEN** a slot has 1 open process, 2 uncommitted files, and 3 untracked files
  and is not locked
- **THEN** the badge SHALL be red `in use` and the stats column SHALL be
  `⚙1 ±2 ?3`

#### Scenario: Locked worktree shows yellow badge

- **WHEN** a pool slot is git-locked and is otherwise clean and idle
- **THEN** the badge SHALL be yellow `locked` and the stats column SHALL be
  blank

#### Scenario: Locked worktree with open processes shows stats

- **WHEN** a pool slot is git-locked and also has 2 open processes
- **THEN** the badge SHALL be yellow `locked` and the stats column SHALL contain
  `⚙2`

#### Scenario: Locked worktree with uncommitted changes shows stats

- **WHEN** a pool slot is git-locked and also has uncommitted changes (e.g.
  `±1`)
- **THEN** the badge SHALL be yellow `locked` and the stats column SHALL contain
  `±1`

#### Scenario: Locked beats in-use — locked and dirty slot shows as locked

- **WHEN** a pool slot is git-locked and also has open processes and uncommitted
  changes
- **THEN** the badge SHALL be yellow `locked` (not red `in use`)
- **THEN** the stats column SHALL show all non-zero values (e.g. `⚙1 ±2`)

#### Scenario: Mixed pool

- **WHEN** the pool contains multiple slots in different states
- **THEN** each slot SHALL appear on its own line with the correct badge, path,
  optional branch, and stats column

#### Scenario: Current slot is marked in the list

- **WHEN** the user runs `bs list` from inside a managed pool slot (e.g.
  `~/.bonsai/repo/a3f9c1b2`)
- **THEN** the row for that slot SHALL be prefixed with `▶`
- **THEN** all other rows SHALL appear with a two-space indent in place of the
  arrow

#### Scenario: Current slot subdirectory is still detected

- **WHEN** the user runs `bs list` from a subdirectory inside a managed pool
  slot (e.g. `~/.bonsai/repo/a3f9c1b2/src`)
- **THEN** the row for the containing slot SHALL be prefixed with `▶`

#### Scenario: CWD is not inside any managed slot

- **WHEN** the user runs `bs list` from a directory that is not inside any
  managed pool slot
- **THEN** no row SHALL be prefixed with `▶`

#### Scenario: `current_worktree()` fails gracefully

- **WHEN** `current_worktree()` returns an error (e.g. git unavailable)
- **THEN** `bs list` SHALL still display all slots without a current indicator,
  without producing an error

### Requirement: Available status means clean, unlocked, and not opened by any process at the slot root

A worktree slot's display status SHALL be determined as follows, in priority
order:

1. **`locked`** (yellow) — if the slot is git-locked, regardless of other
   signals.
2. **`in use`** (red) — if the slot is not locked but has uncommitted changes,
   untracked files, or at least one process with an open file descriptor
   directly in the slot root directory.
3. **`available`** (green) — if the slot is not locked, its working tree is
   clean, and no process has an open handle directly at the slot root.

`lsof +d <slot>` (non-recursive) is used for process detection. Processes with
open handles only in subdirectories of the slot root SHALL NOT cause the slot to
be classified as `in use`. If `lsof` cannot be run, the CLI SHALL exit with a
non-zero status and an actionable error message.

#### Scenario: Locked slot shown as locked

- **WHEN** a pool slot is git-locked
- **THEN** `bs list` SHALL display it with a yellow `locked` badge

#### Scenario: Locked and dirty slot shown as locked (not in use)

- **WHEN** a pool slot is git-locked and also has uncommitted changes
- **THEN** `bs list` SHALL display it with a yellow `locked` badge

#### Scenario: Dirty slot (not locked) shown as in use

- **WHEN** a pool slot has uncommitted changes and is not locked
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Slot with shell CWD at root shown as in use

- **WHEN** a slot is unlocked and clean
- **WHEN** a shell process has CWD = the slot root
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Slot with open handles only in subdirectory shown as available

- **WHEN** a slot is unlocked and clean
- **WHEN** a process has an open file descriptor only in a subdirectory (e.g. an
  editor buffer at `<slot>/src/main.rs`) but NOT in the slot root itself
- **THEN** `bs list` SHALL display it with a green `available` badge

#### Scenario: Clean unlocked slot with no top-level open handles shown as available

- **WHEN** a pool slot is unlocked, its working tree is clean, and no process
  has an open handle directly in its root directory
- **THEN** `bs list` SHALL display it with a green `available` badge
