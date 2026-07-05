## Purpose

Provision and reuse managed git worktrees from a per-repo pool under
`~/.bonsai/<repo-slug>/`, so that ephemeral task environments can be acquired
cheaply without re-running setup steps for every branch.

## Requirements

### Requirement: `get` subcommand exists and is the default

The CLI SHALL expose a `get` subcommand. Running `bs get` SHALL NOT require any
arguments to succeed. Running `bs` with no subcommand SHALL behave identically
to `bs get`.

#### Scenario: Invoke with explicit subcommand

- **WHEN** the user runs `bs get`
- **THEN** the process exits with code 0 and prints the worktree path to stdout

#### Scenario: Invoke without any subcommand

- **WHEN** the user runs `bs` with no subcommand and no arguments
- **THEN** the process behaves as if `bs get` was called

### Requirement: HEAD is resolved from the calling worktree

`bs get` SHALL determine the target commit by running `git rev-parse HEAD` in
the current working directory. This is correct whether the command is invoked
from the main worktree or from a linked worktree.

#### Scenario: Called from the main worktree

- **WHEN** the user runs `bs get` from the main repository working directory
- **THEN** HEAD SHALL be the commit currently checked out in the main worktree

#### Scenario: Called from a linked worktree

- **WHEN** the user runs `bs get` from inside a managed pool worktree (e.g.
  `~/.bonsai/bonsai/a3f9c1b2/`)
- **THEN** HEAD SHALL be the commit checked out in that linked worktree, not the
  main worktree's HEAD

### Requirement: Repo root is resolved via the shared git directory

`bs get` SHALL derive the repo slug using `git rev-parse --git-common-dir` to
locate the shared `.git` directory, taking its parent as the canonical main repo
root. This ensures correct slug derivation regardless of which worktree the
command is called from.

#### Scenario: Called from a linked worktree

- **WHEN** `bs get` is run from inside `~/.bonsai/bonsai/a3f9c1b2/`
- **THEN** the repo slug SHALL be derived from the main repo root (e.g.
  `bonsai`), NOT from the worktree directory name (e.g. `a3f9c1b2`)

#### Scenario: Called from the main worktree

- **WHEN** `bs get` is run from `/Users/alice/repos/bonsai`
- **THEN** the repo slug SHALL be `bonsai`

### Requirement: Pool slots are identified by UUID

All pool slot directories SHALL be named using an 8-character prefix of a UUID
v4 value (e.g. `a3f9c1b2`). Slot names SHALL NOT be derived from branch names or
any user-provided input.

#### Scenario: New slot created

- **WHEN** `bs get` creates a new worktree slot
- **THEN** the slot directory name SHALL be an 8-character UUID prefix
- **THEN** the full path SHALL follow the pattern
  `<root>/<repo-slug>/<uuid-prefix>/`

### Requirement: Pool is scanned for an available slot before creating a new one

Before creating a new worktree, `bs get` SHALL scan all registered worktrees
under the managed pool path. A slot is **available** if and only if: (1) its
directory exists on disk, (2) it is not locked, and (3) its working tree is
clean (`git -C <slot-path> status --porcelain` returns no output).

#### Scenario: Available slot found in pool

- **WHEN** at least one pool slot is not locked and has a clean working tree
- **THEN** `bs get` SHALL reuse that slot rather than creating a new one
- **THEN** `git worktree add` SHALL NOT be called

#### Scenario: All pool slots are unavailable

- **WHEN** every pool slot is either locked or has uncommitted changes
- **THEN** `bs get` SHALL create a new UUID-named slot

#### Scenario: Pool is empty

- **WHEN** no slots exist under the managed pool path
- **THEN** `bs get` SHALL create a new UUID-named slot

### Requirement: Stale registrations are pruned before scanning

`bs get` SHALL run `git worktree prune` before scanning the pool if any slot
appears in `git worktree list` output but its directory no longer exists on
disk.

#### Scenario: Stale registration exists

- **WHEN** a slot is registered in git but its directory has been manually
  deleted
- **WHEN** `bs get` is called
- **THEN** `git worktree prune` SHALL be executed before the pool scan

### Requirement: Available slot is reset to HEAD in detached state

When an available slot is found, `bs get` SHALL reset it to the resolved HEAD
SHA using detached HEAD checkout
(`git -C <slot-path> checkout --detach <HEAD-SHA>`).

#### Scenario: Slot reset to caller's HEAD

- **WHEN** an available slot is found and the resolved HEAD is commit `abc1234`
- **THEN** `git -C <slot-path> checkout --detach abc1234` SHALL be executed
- **THEN** the slot SHALL be in detached HEAD state at `abc1234`

### Requirement: New slot is created in detached HEAD state

When no available slot exists, `bs get` SHALL create a new worktree using
`git worktree add --detach <slot-path> <HEAD-SHA>`.

#### Scenario: New slot created at caller's HEAD

- **WHEN** no available slot is found and the resolved HEAD is commit `abc1234`
- **THEN** `git worktree add --detach <new-slot-path> abc1234` SHALL be executed
- **THEN** the new slot SHALL be in detached HEAD state at `abc1234`

### Requirement: Pool directory structure is created on demand

`bs get` SHALL ensure `~/.bonsai/<repo-slug>/` exists before scanning or
creating slots, using the equivalent of `fs::create_dir_all`. It SHALL NOT error
if the directories already exist.

#### Scenario: Neither `~/.bonsai` nor the repo-slug directory exist

- **WHEN** the user runs `bs get` for the first time on a machine
- **THEN** both `~/.bonsai/` and `~/.bonsai/<repo-slug>/` SHALL be created
- **THEN** the command SHALL proceed normally

#### Scenario: Directories already exist

- **WHEN** `~/.bonsai/<repo-slug>/` already exists
- **WHEN** the user runs `bs get`
- **THEN** no error SHALL occur and the existing directory SHALL be used as-is

### Requirement: Pool root is fixed at `~/.bonsai`

`bs get` SHALL accept no arguments or options. The managed root SHALL always be
`~/.bonsai` and the pool path SHALL always be `~/.bonsai/<repo-slug>/`.

#### Scenario: Default root always used

- **WHEN** the user runs `bs get`
- **THEN** the pool path SHALL be rooted under `~/.bonsai/<repo-slug>/`

### Requirement: Worktree path is printed to stdout

`bs get` SHALL print the absolute path of the provisioned (or reused) worktree
to stdout, prefixed with the 🌳 emoji.

#### Scenario: Path printed for reset slot

- **WHEN** `bs get` reuses and resets an existing slot
- **THEN** stdout SHALL contain the 🌳 emoji followed by the absolute path of
  that slot

#### Scenario: Path printed for new slot

- **WHEN** `bs get` creates a new slot
- **THEN** stdout SHALL contain the 🌳 emoji followed by the absolute path of
  the new slot
