## MODIFIED Requirements

### Requirement: Available status means clean, unlocked, and not opened by any process at the slot root

A worktree slot SHALL be displayed as **available** (green) if and only if: (1)
its directory exists on disk, (2) it is not locked, (3) its working tree is
clean (`git -C <slot> status --porcelain` returns empty output), and (4) no
process currently has an open file descriptor **directly in** the slot root
directory (as determined by `lsof +d <slot>`). Processes with open handles only
in subdirectories of the slot root SHALL NOT cause the slot to be classified as
in use. If `lsof` cannot be run, the CLI SHALL exit with a non-zero status and
an actionable error message.

#### Scenario: Locked slot shown as in use

- **WHEN** a pool slot is locked
- **THEN** `bs list` SHALL display it with a red `in use` badge

#### Scenario: Dirty slot shown as in use

- **WHEN** a pool slot has uncommitted changes
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

## ADDED Requirements

### Requirement: Per-slot status checks are performed concurrently

`bs list` SHALL evaluate per-slot availability concurrently rather than
serially. The `lsof +d` and `git status --porcelain` checks for each slot SHALL
be started in parallel; the display order SHALL match the order returned by
`git worktree list --porcelain` regardless of completion order.

#### Scenario: Multiple slots evaluated without serial blocking

- **WHEN** the pool contains N slots each requiring an `lsof` and `git status`
  call
- **THEN** the wall-clock time SHALL be bounded by the slowest single slot, not
  by the sum of all slots

#### Scenario: Display order preserved

- **WHEN** slots A, B, C are returned by `git worktree list --porcelain` in that
  order
- **WHEN** slot C finishes its checks before slot A
- **THEN** `bs list` SHALL still print slot A first, then B, then C
