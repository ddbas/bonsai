## MODIFIED Requirements

### Requirement: Each worktree is shown on its own line with path, branch, and status badge

`bs list` SHALL print one line per managed pool worktree. Each line SHALL
contain:

1. When the slot is the one that contains the process's current working
   directory, the line SHALL be prefixed with `▶`, so that the active slot is
   visually distinct from the rest. All other lines SHALL be prefixed with two
   spaces instead. The `▶` prefix alone is sufficient to indicate the current
   slot; no additional `(current)` label SHALL be printed anywhere on the line.
2. A colored status badge — `available`, `in use`, or `locked` — reflecting the
   slot's classification, computed using the same priority rules as `bs status`
   (`locked` > `in use` > `available`), printed **before** the worktree path.
   The badge SHALL be left-aligned and right-padded (based on its plain,
   uncolored text) to a fixed column width equal to the length of the longest
   possible badge string (`available`, 9 characters), so that the worktree path
   column begins at the same screen column on every line regardless of which
   badge is shown on that line. Color codes applied to the badge SHALL NOT
   affect the padding width calculation.
3. The worktree path (with home directory prefix replaced with `~`), starting at
   the same fixed column on every line.
4. Optionally, the checked-out branch name in **bold parentheses** immediately
   after the path (omitted for detached HEAD).

`bs list` SHALL NOT display a usage-stats column (`⚙N ±N ?N`). Detailed per-slot
status — itemized lock reason, uncommitted/untracked files, and open processes —
is available via `bs status <path>` instead; `bs list`'s badge is limited to the
three-way classification.

#### Scenario: Single worktree, detached HEAD

- **WHEN** the pool contains one slot in detached HEAD state
- **THEN** stdout SHALL contain one line with the status badge, followed by the
  tilde-prefixed path, no branch suffix, and no stats column

#### Scenario: Single worktree with a branch

- **WHEN** the pool contains one slot with branch `main` checked out
- **THEN** stdout SHALL contain one line with the status badge, followed by the
  tilde-prefixed path and `(main)` in bold, and no stats column

#### Scenario: Mixed pool

- **WHEN** the pool contains multiple slots in different lock/dirty/open-process
  states
- **THEN** each slot SHALL appear on its own line with its path, optional
  branch, and status badge reflecting its own classification — no stats column
  SHALL appear for any slot

#### Scenario: Current slot is marked in the list

- **WHEN** the user runs `bs list` from inside a managed pool slot (e.g.
  `~/.bonsai/repo/a3f9c1b2`)
- **THEN** the row for that slot SHALL be prefixed with `▶`
- **THEN** all other rows SHALL appear without a `▶` prefix

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

#### Scenario: Path column is aligned across badges of different lengths

- **WHEN** the pool contains at least one slot classified `available` (badge
  text `"available"`, 9 characters) and at least one slot classified `in use` or
  `locked` (badge text 6 characters)
- **THEN** the worktree path SHALL begin at the same character column on every
  printed line, regardless of that line's badge text length

#### Scenario: Path column is aligned when the current-slot marker is present

- **WHEN** the pool contains slots with different badge lengths and one of them
  is prefixed with `▶` because it is the current slot
- **THEN** the worktree path SHALL still begin at the same character column on
  every printed line, since the `▶`/two-space prefix width is constant across
  all rows and only the badge padding varies
