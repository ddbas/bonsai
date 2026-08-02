## MODIFIED Requirements

### Requirement: Each worktree is shown on its own line with path and branch only

`bs list` SHALL print one line per managed pool worktree. Each line SHALL
contain:

1. The worktree path (with home directory prefix replaced with `~`).
2. Optionally, the checked-out branch name in **bold parentheses** immediately
   after the path (omitted for detached HEAD).
3. When the slot is the one that contains the process's current working
   directory, the line SHALL be prefixed with `▶` and annotated with `(current)`
   immediately after the branch (or path when no branch is present), so that the
   active slot is visually distinct from the rest. All other lines SHALL retain
   their existing format without any prefix.

`bs list` SHALL NOT display a status badge (`available` / `in use` / `locked`)
or a usage-stats column (`⚙N ±N ?N`). `bs list` SHALL NOT invoke `lsof` or
`git status --porcelain` for any slot; detailed per-slot status (lock state,
uncommitted/untracked files, open processes) is available via `bs status <path>`
instead.

#### Scenario: Single worktree, detached HEAD

- **WHEN** the pool contains one slot in detached HEAD state
- **THEN** stdout SHALL contain one line with the tilde-prefixed path and no
  branch suffix, and no status badge or stats column

#### Scenario: Single worktree with a branch

- **WHEN** the pool contains one slot with branch `main` checked out
- **THEN** stdout SHALL contain one line with the tilde-prefixed path followed
  by `(main)` in bold, and no status badge or stats column

#### Scenario: Mixed pool

- **WHEN** the pool contains multiple slots regardless of their lock state,
  dirty state, or open processes
- **THEN** each slot SHALL appear on its own line with only its path and
  optional branch — no badge or stats column SHALL appear for any slot

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

### Requirement: `bs list` does not perform per-slot availability checks

`bs list` SHALL enumerate pool slots using only `git worktree list --porcelain`
(a single git invocation). It SHALL NOT spawn `lsof`, SHALL NOT run
`git status --porcelain` per slot, and SHALL NOT spawn per-slot threads for
availability classification. Its cost SHALL NOT scale with the number of open
processes or dirty files in any slot.

#### Scenario: Listing does not shell out to `lsof`

- **WHEN** the user runs `bs list` against a pool with N slots
- **THEN** `bs list` SHALL NOT invoke `lsof` for any slot

#### Scenario: Listing does not run `git status` per slot

- **WHEN** the user runs `bs list` against a pool with N slots
- **THEN** `bs list` SHALL NOT invoke `git status --porcelain` for any slot

## REMOVED Requirements

### Requirement: Available status means clean, unlocked, and not opened by any process at the slot root

**Reason**: `bs list` no longer computes or displays per-slot availability
status; enumerating a pool no longer requires per-slot `lsof`/`git status`
checks. This classification logic is preserved, unchanged in its priority rules,
under `bs status` (see `specs/worktree-status/spec.md`).

**Migration**: Use `bs status <path>` (or `bs status` from inside the slot) to
see whether a specific slot is `locked`, `in use`, or `available`.

### Requirement: Per-slot status checks are performed concurrently

**Reason**: `bs list` no longer performs per-slot status checks at all, so
concurrency across slots during `list` is no longer applicable. `bs status` only
ever inspects one slot per invocation, so no cross-slot concurrency is needed
there either.

**Migration**: No action needed; `bs list` is now fast without concurrency
tricks. Running `bs status` for multiple slots (e.g. in a shell loop) is the
replacement if a user needs status for several slots at once.
