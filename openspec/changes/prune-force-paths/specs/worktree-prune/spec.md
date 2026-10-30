## ADDED Requirements

### Requirement: `prune` accepts optional worktree paths

`bs prune` SHALL accept zero or more optional positional worktree path
arguments. When one or more paths are given, `bs prune` SHALL prune exactly
those slots and SHALL NOT delete, preserve, detach, or otherwise modify any
other slot; the "At least one available slot is always preserved" requirement
does not apply. Relative paths SHALL be resolved against the current working
directory. When no paths are given, `bs prune` SHALL behave exactly as
specified by the other requirements in this spec.

#### Scenario: Prune a single available slot by path

- **WHEN** the user runs `bs prune <path>` where `<path>` is an `available` slot
  and the pool contains other available slots
- **THEN** only that slot's directory is deleted, it is reported as pruned with
  its path and branch, the other slots are untouched (none is preserved or
  detached), `git worktree prune` runs, and the process exits with code 0

#### Scenario: Prune several slots by path

- **WHEN** the user runs `bs prune <path-a> <path-b>` where both are `available`
- **THEN** both slots' directories are deleted and reported as pruned

#### Scenario: Path is not a managed slot

- **WHEN** the user runs `bs prune <path>` where `<path>` is not a bonsai pool
  slot of the current repository
- **THEN** the process exits non-zero with an error naming the path, and nothing
  is deleted

### Requirement: `--force` allows pruning `in use` slots

`bs prune` SHALL accept a `--force` option. When paths are given, a targeted
slot classified `in use` SHALL be pruned only if `--force` is passed; without
`--force`, targeting an `in use` slot SHALL be an error stating that `--force`
is required. With `--force`, `in use` slots (including those with uncommitted
changes or unmerged branches) SHALL have their directories deleted and be
reported as pruned like any other pruned slot. `--force` SHALL NOT be accepted
without at least one path; doing so SHALL be an error and nothing SHALL be
deleted.

#### Scenario: Force-prune an in-use slot

- **WHEN** the user runs `bs prune --force <path>` where `<path>` is an `in use`
  slot
- **THEN** that slot's directory is deleted, it is reported as pruned, `git
  worktree prune` runs, and the process exits with code 0

#### Scenario: In-use slot targeted without force

- **WHEN** the user runs `bs prune <path>` where `<path>` is an `in use` slot
- **THEN** the process exits non-zero with an error that `--force` is required,
  and the slot's directory is not deleted

#### Scenario: Force without paths

- **WHEN** the user runs `bs prune --force` with no path arguments
- **THEN** the process exits non-zero with an error that `--force` requires one
  or more paths, and nothing is deleted

### Requirement: Locked slots can never be pruned

`bs prune` SHALL refuse to prune a slot classified `locked`, regardless of
whether `--force` is passed. Targeting a locked slot SHALL be an error whose
message states the slot is locked and that it must be unlocked (`bs unlock`)
before it can be pruned. The slot's directory and registration SHALL NOT be
modified.

#### Scenario: Locked slot targeted without force

- **WHEN** the user runs `bs prune <path>` where `<path>` is a locked slot
- **THEN** the process exits non-zero with an error saying the slot is locked
  and must be unlocked first, and the slot is not deleted

#### Scenario: Locked slot targeted with force

- **WHEN** the user runs `bs prune --force <path>` where `<path>` is a locked
  slot
- **THEN** the process exits non-zero with the same locked error, and the slot
  is not deleted

#### Scenario: Pruned after unlocking

- **WHEN** the user runs `bs unlock <path>` and then `bs prune --force <path>`
  on a slot that was locked and in use
- **THEN** the slot is pruned

### Requirement: Targets are validated before any deletion

When paths are given, `bs prune` SHALL classify and validate every target
before deleting anything. If any target is not a managed slot, is `locked`, or
is `in use` without `--force`, `bs prune` SHALL report every offending target,
SHALL delete nothing, and SHALL exit non-zero. Once all targets are valid,
per-slot deletion failures SHALL be handled as specified by "Per-slot deletion
failures do not abort the whole run".

#### Scenario: One invalid target among valid ones

- **WHEN** the user runs `bs prune --force <available> <locked>`
- **THEN** the locked error is reported, neither slot is deleted, and the
  process exits non-zero

#### Scenario: Duplicate paths

- **WHEN** the same slot path is passed more than once
- **THEN** the slot is pruned once
