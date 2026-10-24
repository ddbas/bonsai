## ADDED Requirements

### Requirement: Pool is scanned for a git-state-available slot before creating a new one

Before creating a new worktree, `bs get` SHALL scan all registered worktrees
under the managed pool path. A slot is **available** if and only if: (1) its
directory exists on disk, (2) it is not locked, (3) its working tree is clean
(`git -C <slot-path> status --porcelain --untracked-files=all` returns no
output), and (4) its branch state carries no unmerged work (see
`worktree-branch-state`). Open processes in the slot SHALL NOT affect
availability.

#### Scenario: Available slot found in pool

- **WHEN** at least one pool slot is not locked, has a clean working tree, and
  has no unmerged work
- **THEN** `bs get` SHALL reuse that slot rather than creating a new one
- **THEN** `git worktree add` SHALL NOT be called

#### Scenario: All pool slots are unavailable

- **WHEN** every pool slot is either locked, has uncommitted changes, or has
  unmerged branch work
- **THEN** `bs get` SHALL create a new UUID-named slot

#### Scenario: Pool is empty

- **WHEN** no slots exist under the managed pool path
- **THEN** `bs get` SHALL create a new UUID-named slot

#### Scenario: Slot with open file handles is still reused

- **WHEN** a pool slot is unlocked, clean, has no unmerged work, and a process
  has its CWD at the slot root
- **THEN** `bs get` SHALL reuse that slot

#### Scenario: Slot with unmerged work is not reused

- **WHEN** a pool slot is unlocked and clean but its branch has commits not on
  the default ref
- **THEN** `bs get` SHALL NOT reuse that slot
- **THEN** `bs get` SHALL continue scanning remaining slots or create a new one

## REMOVED Requirements

### Requirement: Pool is scanned for an available slot before creating a new one

**Reason**: Availability no longer depends on open processes. **Migration**:
Replaced by "Pool is scanned for a git-state-available slot before creating a
new one".
