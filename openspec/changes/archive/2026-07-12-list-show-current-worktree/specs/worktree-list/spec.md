## MODIFIED Requirements

### Requirement: Each worktree is shown on its own line with path, branch, status, and usage stats

`bs list` SHALL print one line per managed pool worktree. Each line SHALL
contain:

1. A coloured status badge (`available` in green, `in use` in red).
2. The worktree path (with home directory prefix replaced with `~`).
3. Optionally, the checked-out branch name in **bold parentheses** immediately
   after the path (omitted for detached HEAD).
4. A compact usage-stats column that shows only non-zero values for: open
   processes (`⚙N`), uncommitted files (`±N`), and untracked files (`?N`),
   space-separated. The column is blank for clean idle slots.
5. When the slot is the one that contains the process's current working
   directory, the line SHALL be prefixed with `▶` and annotated with `(current)`
   immediately after the branch (or path when no branch is present), so that the
   active slot is visually distinct from the rest. All other lines SHALL retain
   their existing format without any prefix.

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

- **WHEN** a slot has 2 open processes
- **THEN** the stats column SHALL contain `⚙2`

#### Scenario: In-use worktree with uncommitted changes only

- **WHEN** a slot has 3 modified/staged files and no open processes or untracked
  files
- **THEN** the stats column SHALL contain `±3`

#### Scenario: In-use worktree with all three stats

- **WHEN** a slot has 1 open process, 2 uncommitted files, and 3 untracked files
- **THEN** the stats column SHALL be `⚙1 ±2 ?3`

#### Scenario: Mixed pool

- **WHEN** the pool contains multiple slots in different states
- **THEN** each slot SHALL appear on its own line with the correct badge, path,
  optional branch, and stats column

#### Scenario: Current slot is marked in the list

- **WHEN** the user runs `bs list` from inside a managed pool slot (e.g.
  `~/.bonsai/repo/a3f9c1b2`)
- **THEN** the row for that slot SHALL be prefixed with `▶` and SHALL include
  `(current)` after the branch (or path)
- **THEN** all other rows SHALL appear without a `▶` prefix

#### Scenario: Current slot subdirectory is still detected

- **WHEN** the user runs `bs list` from a subdirectory inside a managed pool
  slot (e.g. `~/.bonsai/repo/a3f9c1b2/src`)
- **THEN** the row for the containing slot SHALL be prefixed with `▶` and
  annotated with `(current)`

#### Scenario: CWD is not inside any managed slot

- **WHEN** the user runs `bs list` from a directory that is not inside any
  managed pool slot
- **THEN** no row SHALL be prefixed with `▶` and no `(current)` label SHALL
  appear

#### Scenario: `current_worktree()` fails gracefully

- **WHEN** `current_worktree()` returns an error (e.g. git unavailable)
- **THEN** `bs list` SHALL still display all slots without a current indicator,
  without producing an error
