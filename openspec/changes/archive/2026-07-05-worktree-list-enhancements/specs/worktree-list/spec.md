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
