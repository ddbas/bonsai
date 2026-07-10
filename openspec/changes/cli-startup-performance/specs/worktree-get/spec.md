## MODIFIED Requirements

### Requirement: HEAD and repo root are resolved in a single git subprocess

`bs get` SHALL determine the target commit SHA and the canonical repo root in a
single `git rev-parse HEAD --git-common-dir` subprocess invocation, parsing the
two output lines individually. Spawning separate `git rev-parse HEAD` and
`git rev-parse --git-common-dir` subprocesses for the same invocation of
`bs get` is NOT permitted.

#### Scenario: Single subprocess returns both values

- **WHEN** the user runs `bs get`
- **THEN** exactly one `git rev-parse HEAD --git-common-dir` subprocess SHALL be
  spawned to obtain the HEAD SHA and the common git directory path
- **THEN** the HEAD SHA and repo slug SHALL be derived from that single call

### Requirement: Pool is scanned for an available slot before creating a new one

Before creating a new worktree, `bs get` SHALL scan all registered worktrees
under the managed pool path. A slot is **available** if and only if: (1) its
directory exists on disk, (2) it is not locked, (3) its working tree is clean
(`git -C <slot-path> status --porcelain` returns no output), and (4) no process
currently has an open file descriptor **directly in** the slot root directory
(as determined by `lsof +d <slot-path>`). Processes with open handles only in
subdirectories of the slot SHALL NOT cause the slot to be classified as in use.
If `lsof` cannot be run, the CLI SHALL exit with a non-zero status and an
actionable error message.

#### Scenario: Available slot found in pool

- **WHEN** at least one pool slot is not locked, has a clean working tree, and
  no process has a file open directly in its root directory
- **THEN** `bs get` SHALL reuse that slot rather than creating a new one
- **THEN** `git worktree add` SHALL NOT be called

#### Scenario: All pool slots are unavailable

- **WHEN** every pool slot is either locked, has uncommitted changes, or has a
  process with an open handle directly in its root
- **THEN** `bs get` SHALL create a new UUID-named slot

#### Scenario: Pool is empty

- **WHEN** no slots exist under the managed pool path
- **THEN** `bs get` SHALL create a new UUID-named slot

#### Scenario: Slot with open file handles is not reused

- **WHEN** a pool slot is unlocked and has a clean working tree
- **WHEN** a shell process has CWD = the slot root
- **THEN** `bs get` SHALL NOT reuse that slot
- **THEN** `bs get` SHALL continue scanning remaining slots or create a new one

### Requirement: Stale registrations are pruned only when detected

`bs get` SHALL run `git worktree prune` before scanning the pool **only if** the
output of `git worktree list --porcelain` includes at least one registered
worktree path that does not exist on disk. When all registered worktree paths
exist, `git worktree prune` SHALL NOT be invoked.

#### Scenario: All registered worktrees present — no prune

- **WHEN** every path listed by `git worktree list --porcelain` exists on disk
- **WHEN** `bs get` is called
- **THEN** `git worktree prune` SHALL NOT be executed

#### Scenario: Stale registration exists — prune runs

- **WHEN** a slot is registered in git but its directory has been manually
  deleted
- **WHEN** `bs get` is called
- **THEN** `git worktree prune` SHALL be executed before the pool scan
