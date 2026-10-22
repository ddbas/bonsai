## REMOVED Requirements

### Requirement: `has_open_files` detects open file handles in the top-level slot directory

**Reason**: Availability is now determined from git state, not open processes.
**Migration**: Use `bs lock` to protect slots with active work.

### Requirement: `lsof` absence causes the CLI to exit with a hard error

**Reason**: `lsof` is no longer used. **Migration**: None; `lsof` is no longer a
dependency.

### Requirement: `lsof` failure on a specific path causes `has_open_files` to return an error

**Reason**: `lsof` is no longer used. **Migration**: None.

### Requirement: A slot with open file handles at the slot root is classified as `InUse`

**Reason**: Open processes no longer affect classification. **Migration**: Use
`bs lock` to protect slots with active work.

### Requirement: A slot with no open file handles may still be classified as `Available`

**Reason**: Superseded by branch-state classification (see
`worktree-branch-state`). **Migration**: None.

### Requirement: `has_open_files` errors propagate as hard failures

**Reason**: `lsof` is no longer used. **Migration**: None.

### Requirement: `count_open_processes` returns the number of distinct PIDs with open handles in the top-level slot directory

**Reason**: Process listing is removed from `bs status`. **Migration**: None.
