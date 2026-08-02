## Purpose

Provide a `bs list` subcommand (alias `bs ls`) that enumerates all
bonsai-managed worktrees for the current repository's pool and displays each
slot with a colour-coded availability badge, making it easy to inspect pool
state without parsing raw git output. Detailed per-slot status (itemized lock
reason, uncommitted/untracked files, and open processes) is available via
`bs status <path>`.

## Requirements

### Requirement: `list` subcommand exists with `ls` alias

The CLI SHALL expose a `list` subcommand. Running `bs list` SHALL be equivalent
to running `bs ls`.

#### Scenario: Invoke with `list`

- **WHEN** the user runs `bs list`
- **THEN** the process exits with code 0 and prints the worktree list to stdout

#### Scenario: Invoke with `ls` alias

- **WHEN** the user runs `bs ls`
- **THEN** the process behaves identically to `bs list`

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

### Requirement: `bs list` short-circuits per-slot availability checks

For each slot, `bs list` SHALL determine the status badge using early-return
short-circuiting, evaluating signals in increasing order of cost and stopping as
soon as the classification is determined:

1. Lock state (free — from the already-parsed `git worktree list --porcelain`
   output). If locked, `bs list` SHALL classify the slot `locked` **without**
   invoking `git status` or `lsof` for that slot.
2. Dirty/untracked file state (`git status --porcelain`, boolean check only — no
   full parse of individual lines is required). If dirty and unlocked, `bs list`
   SHALL classify the slot `in use` **without** invoking `lsof` for that slot.
3. Open-process state (`lsof -w +d <slot>`, boolean check only). Evaluated only
   when the slot is unlocked and clean, to distinguish `in use` (open processes
   present) from `available`.

`bs list` SHALL NOT compute or display per-file or per-process detail (counts or
itemized lists); it SHALL NOT spawn per-slot threads for anything beyond this
three-signal classification.

#### Scenario: Locked slot skips git status and lsof

- **WHEN** a pool slot is git-locked
- **THEN** `bs list` SHALL NOT invoke `git status --porcelain` or `lsof` for
  that slot

#### Scenario: Dirty unlocked slot skips lsof

- **WHEN** a pool slot is unlocked and has uncommitted or untracked files
- **THEN** `bs list` SHALL NOT invoke `lsof` for that slot

#### Scenario: Clean unlocked slot requires an lsof check

- **WHEN** a pool slot is unlocked and has no uncommitted or untracked files
- **THEN** `bs list` SHALL invoke `lsof` for that slot to distinguish `in use`
  from `available`

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

### Requirement: `bs list` meets a documented performance SLO, calibrated per slot-state scenario

`bs list`'s per-slot classification latency SHALL be tracked and enforced by an
automated benchmark (`benches/bs_ls.rs`, run via `mise run bench` /
`scripts/check-bs-ls-perf.sh`), measured as p95 latency over repeated in-process
invocations of the classification path on a warm filesystem cache, with SLOs
calibrated separately per slot-state scenario since the achievable bound depends
on how many slots require an `lsof`/`git status` call:

1. **All slots locked, or all slots dirty/unlocked** (no or partial subprocess
   fan-out): at a pool size of 50 managed worktree slots, p95 latency SHALL be
   **<= 100ms**, and the scaling ratio of p95 at 50 slots vs. p95 at 5 slots
   SHALL be **<= 6.0x**.
2. **All slots clean, unlocked, and available** (every slot requires both a
   `git status` and an `lsof` call — the classification cost floor, identical in
   shape to the pre-change per-slot fan-out this change's early return does not
   eliminate for this scenario): latency is tracked and reported by the
   benchmark but is **not** subject to a fixed absolute bound; instead, it is
   checked against a scaling-regression bound (p95 at 50 slots SHALL NOT exceed
   the pre-change `list_worktrees_status` baseline ratio by more than a
   documented margin), to catch a regression beyond the expected cost of one
   `lsof` + one `git status` call per slot, without demanding sub-linear scaling
   that isn't achievable when every slot must be checked.

This benchmark and its thresholds exist specifically to catch (a) a regression
back to per-slot subprocess fan-out for scenarios where early return should have
avoided it (scenario 1), and (b) an unexpected additional cost beyond the
inherent `lsof`/`git status` floor for the all-available scenario (scenario 2).

#### Scenario: CI fails on an SLO violation in the locked/dirty scenarios

- **WHEN** `scripts/check-bs-ls-perf.sh` is run (locally via `mise run bench`,
  in the `pre-commit` git hook when `src/**/*.rs` or `benches/**/*.rs` change,
  or in CI on every push/PR) against the all-locked or all-dirty benchmark
  scenarios
- **THEN** it SHALL exit non-zero and print which SLO was violated (absolute p95
  bound, scaling ratio bound, or both) if either threshold above is exceeded
- **THEN** it SHALL exit zero and print the measured p95 values when both
  thresholds are met

#### Scenario: All-available scenario is reported but not hard-gated on an absolute bound

- **WHEN** `scripts/check-bs-ls-perf.sh` is run against the all-available
  benchmark scenario
- **THEN** it SHALL print the measured p95 values at each pool size
- **THEN** it SHALL fail only if the measured cost regresses beyond the
  documented pre-change baseline margin, not merely for being slower than the
  locked/dirty scenarios' absolute bound

### Requirement: Empty pool prints a friendly message

If no worktree slots exist in the pool, `bs list` SHALL print a human-readable
message to stdout indicating the pool is empty rather than printing nothing or
returning an error.

#### Scenario: Pool directory does not exist

- **WHEN** `~/.bonsai/<repo-slug>/` does not exist
- **THEN** `bs list` exits with code 0 and prints a message indicating no
  worktrees are managed for this repository

#### Scenario: Pool directory exists but is empty

- **WHEN** `~/.bonsai/<repo-slug>/` exists but contains no registered worktrees
- **THEN** `bs list` exits with code 0 and prints a message indicating no
  worktrees are managed for this repository

### Requirement: Home directory prefix is displayed as `~`

Paths displayed by `bs list` SHALL have the user's home directory prefix
replaced with `~` for readability.

#### Scenario: Path under home directory

- **WHEN** a slot path is `/Users/alice/.bonsai/myrepo/a3f9c1b2`
- **WHEN** the home directory is `/Users/alice`
- **THEN** the displayed path SHALL be `~/.bonsai/myrepo/a3f9c1b2`

#### Scenario: Path not under home directory

- **WHEN** a slot path does not start with the home directory
- **THEN** the full absolute path SHALL be displayed unchanged
