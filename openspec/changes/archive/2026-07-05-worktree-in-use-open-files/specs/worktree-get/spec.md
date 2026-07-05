## MODIFIED Requirements

### Requirement: Pool is scanned for an available slot before creating a new one

Before creating a new worktree, `bs get` SHALL scan all registered worktrees
under the managed pool path. A slot is **available** if and only if: (1) its
directory exists on disk, (2) it is not locked, (3) its working tree is clean
(`git -C <slot-path> status --porcelain` returns no output), and (4) no process
currently has an open file handle anywhere inside the slot directory (as
determined by `lsof +D <slot-path>`). If `lsof` cannot be run, the slot SHALL be
treated as unavailable.

#### Scenario: Available slot found in pool

- **WHEN** at least one pool slot is not locked, has a clean working tree, and
  no process has it open
- **THEN** `bs get` SHALL reuse that slot rather than creating a new one
- **THEN** `git worktree add` SHALL NOT be called

#### Scenario: All pool slots are unavailable

- **WHEN** every pool slot is either locked, has uncommitted changes, or has
  open file handles
- **THEN** `bs get` SHALL create a new UUID-named slot

#### Scenario: Pool is empty

- **WHEN** no slots exist under the managed pool path
- **THEN** `bs get` SHALL create a new UUID-named slot

#### Scenario: Slot with open file handles is not reused

- **WHEN** a pool slot is unlocked and has a clean working tree
- **WHEN** a process has a file open inside that slot directory
- **THEN** `bs get` SHALL NOT reuse that slot
- **THEN** `bs get` SHALL continue scanning remaining slots or create a new one
