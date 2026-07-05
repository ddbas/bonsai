## Purpose

Show at-a-glance usage statistics (open processes, uncommitted files, untracked
files) for each pool worktree in `bs list`, so users can quickly assess the
state of each slot without running additional git or lsof commands.

## Requirements

### Requirement: Usage stats column shows open-process count with gear icon

`bs list` SHALL render the number of distinct processes with open file handles
inside a slot as `⚙N` (where N ≥ 1) in the stats column. When the count is zero,
the `⚙` component SHALL be omitted.

#### Scenario: Two processes have files open

- **WHEN** exactly 2 distinct processes have open file handles inside a slot
- **THEN** the stats column for that slot SHALL contain `⚙2`

#### Scenario: No processes have files open

- **WHEN** no process has open file handles inside a slot
- **THEN** the stats column SHALL NOT contain a `⚙` component for that slot

### Requirement: Usage stats column shows uncommitted-file count with plus-minus icon

`bs list` SHALL render the number of modified or staged files (lines in
`git status --porcelain` where the two-character XY code is not `??`) as `±N`
(where N ≥ 1) in the stats column. When the count is zero, the `±` component
SHALL be omitted.

#### Scenario: Three files are modified or staged

- **WHEN** `git status --porcelain` reports 3 lines with non-`??` XY codes for a
  slot
- **THEN** the stats column for that slot SHALL contain `±3`

#### Scenario: No uncommitted changes

- **WHEN** `git status --porcelain` reports no modified or staged files for a
  slot
- **THEN** the stats column SHALL NOT contain a `±` component for that slot

### Requirement: Usage stats column shows untracked-file count with question-mark icon

`bs list` SHALL render the number of untracked files (lines in
`git status --porcelain` where the XY code is `??`) as `?N` (where N ≥ 1) in the
stats column. When the count is zero, the `?` component SHALL be omitted.

#### Scenario: Two untracked files

- **WHEN** `git status --porcelain` reports 2 lines with `??` XY code for a slot
- **THEN** the stats column for that slot SHALL contain `?2`

#### Scenario: No untracked files

- **WHEN** `git status --porcelain` reports no untracked files for a slot
- **THEN** the stats column SHALL NOT contain a `?` component for that slot

### Requirement: Clean idle slot SHALL display an empty stats column

The stats column SHALL be blank for any slot that has zero open processes, zero
uncommitted files, and zero untracked files. No icon component SHALL be emitted.

#### Scenario: Available slot

- **WHEN** a slot is clean, unlocked, and has no open processes or untracked
  files
- **THEN** the stats column for that slot SHALL be blank

### Requirement: Non-zero stat components are separated by a space

When two or more stat components are non-zero they SHALL be rendered
space-separated in the order: `⚙N ±N ?N`.

#### Scenario: All three stats non-zero

- **WHEN** a slot has 1 open process, 2 uncommitted files, and 3 untracked files
- **THEN** the stats column SHALL be `⚙1 ±2 ?3`

#### Scenario: Only processes and untracked

- **WHEN** a slot has 2 open processes and 4 untracked files but no uncommitted
  changes
- **THEN** the stats column SHALL be `⚙2 ?4`
