## ADDED Requirements

### Requirement: Report classification uses lock, dirty, and branch state with the same priority as `bs list`

The report SHALL classify the slot as one of `locked`, `in use`, or `available`,
using the same priority order `bs list`'s status badge uses - both are computed
by one shared classification function so the two commands cannot disagree on a
given slot:

1. **`locked`** - if the slot is git-locked, regardless of other signals.
2. **`in use`** - if the slot is not locked but has uncommitted changes,
   untracked files, or a branch state carrying unmerged work (see
   `worktree-branch-state`).
3. **`available`** - if the slot is not locked, its working tree is clean, and
   its branch carries no unmerged work.

Open processes SHALL NOT affect the classification.

#### Scenario: Locked slot classified as locked regardless of dirty state

- **WHEN** the resolved slot is git-locked and also has uncommitted changes and
  unmerged commits
- **THEN** the report SHALL classify the slot as `locked`

#### Scenario: Dirty unlocked slot classified as in use

- **WHEN** the resolved slot is unlocked and has uncommitted changes
- **THEN** the report SHALL classify the slot as `in use`

#### Scenario: Clean slot with unmerged work classified as in use

- **WHEN** the resolved slot is unlocked and clean but its branch has commits
  not on the default ref and not proven merged
- **THEN** the report SHALL classify the slot as `in use`

#### Scenario: Clean slot with no unmerged work classified as available

- **WHEN** the resolved slot is unlocked, clean, and its branch carries no
  unmerged work
- **THEN** the report SHALL classify the slot as `available`

### Requirement: Report includes a branch-state line explaining the classification

The report SHALL include a "Branch state" line stating why the branch is or is
not considered to carry unmerged work, using the reason defined in
`worktree-branch-state` (e.g. "3 commits ahead of origin/main, no upstream
configured", "remote branch still exists", "default branch could not be
determined", "no commits of its own"). When the reason is that commits were
never pushed with upstream tracking, the line SHALL hint that pushing with `-u`
enables merged-branch detection.

#### Scenario: Unmerged commits, never pushed

- **WHEN** the resolved slot's branch has 3 commits not on `origin/main` and no
  upstream config
- **THEN** the report SHALL show a branch-state line stating 3 commits ahead of
  `origin/main` and that no upstream is configured

#### Scenario: Merged and remote gone

- **WHEN** the resolved slot's branch was squash-merged and its remote branch is
  gone
- **THEN** the report SHALL show a branch-state line stating the branch is
  merged and its remote branch is gone

## REMOVED Requirements

### Requirement: Report includes an overall classification using the same priority rules `bs list` used

**Reason**: Branch-state replaces the open-process signal. **Migration**:
Replaced by "Report classification uses lock, dirty, and branch state with the
same priority as `bs list`".

### Requirement: Report lists individual open processes with PID and command name

**Reason**: Process detection via `lsof` is removed. **Migration**: None; use
`bs lock` to protect slots with active work.

### Requirement: lsof unavailability is a hard error

**Reason**: `lsof` is no longer used. **Migration**: None.
