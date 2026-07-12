## Why

When provisioning a worktree for a specific task, developers often want to
immediately begin work on a named branch rather than having to `git checkout -b`
inside the slot after the fact. Today `bs get` always produces a detached HEAD
worktree, forcing a manual branch-creation step every time the caller needs to
commit work.

## What Changes

- Add a `-b <branch>` option to `bs get`: create and check out a new branch
  named `<branch>` at the current HEAD inside the provisioned slot (errors if
  the branch already exists, matching `git checkout -b` semantics).
- Add a `-B <branch>` option to `bs get`: create or reset the branch named
  `<branch>` to HEAD inside the provisioned slot (overwrites an existing branch
  without error, matching `git checkout -B` semantics).
- When neither option is given, behaviour is unchanged: the slot is left in
  detached HEAD state.
- The branch name is printed alongside the worktree path in the output line
  (e.g. `🌳 /path/to/slot  (my-feature)`).

## Capabilities

### New Capabilities

- `get-branch-options`: Accept `-b` and `-B` flags on `bs get` to provision a
  worktree with a named branch already checked out, mirroring `git checkout -b`
  / `git checkout -B` semantics.

### Modified Capabilities

- `worktree-get`: The `get` subcommand now optionally accepts `-b`/`-B`
  arguments; the requirement that it accepts _no_ arguments/options is relaxed.

## Impact

- `src/main.rs`: Add `-b`/`-B` fields to the `Get` variant of `Commands`.
- `src/worktree/mod.rs`: Add `create_slot_with_branch` and
  `reset_slot_with_branch` helpers (or extend existing ones) that run
  `git checkout -b`/`-B` after placing the slot.
- `get_worktree` signature gains an optional `branch: Option<BranchMode>`
  parameter.
- No breaking changes to the pool layout or `~/.bonsai` directory structure.
- New integration tests required (container-isolated via `GitEnv`).
