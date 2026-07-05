## MODIFIED Requirements

### Requirement: Available status means clean, unlocked, and not opened by any process

A worktree slot SHALL be displayed as **available** (green) if and only if: (1)
its directory exists on disk, (2) it is not locked, (3) its working tree is
clean (`git -C <slot> status --porcelain` returns empty output), and (4) no
process currently has an open file handle anywhere inside the slot directory (as
determined by `lsof +D <slot>`). If `lsof` cannot be run, the slot SHALL be
treated as **in use**.

#### Scenario: Locked slot shown as in use

- **WHEN** a pool slot is locked
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Dirty slot shown as in use

- **WHEN** a pool slot has uncommitted changes
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Slot with open file handles shown as in use

- **WHEN** a pool slot is unlocked and clean
- **WHEN** a process has a file open inside the slot directory
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Clean unlocked slot with no open handles shown as available

- **WHEN** a pool slot is unlocked, its working tree is clean, and no process
  has it open
- **THEN** `bs list` SHALL display it with a green `available` badge
