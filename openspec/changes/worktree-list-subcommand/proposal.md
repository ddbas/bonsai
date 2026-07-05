## Why

Users need a quick way to inspect the state of their bonsai worktrees — which
are available to check out and which are already in use — without having to dig
into git internals or manually parse paths. A dedicated `list` subcommand
surfaces this information in a clear, colour-coded format directly in the
terminal.

## What Changes

- Add a `bs list` subcommand (also accessible as `bs ls`) that enumerates all
  git worktrees managed by bonsai.
- Each worktree entry shows its path (with `~` substituted for the home
  directory) and a coloured status indicator: **green** for available, **red**
  for in use.
- "In use" is determined by whether the worktree is currently checked out in an
  active branch (i.e. not bare and not in a detached HEAD state that is
  unoccupied). For bonsai's model this means the worktree has an associated
  branch that is checked out somewhere.

## Capabilities

### New Capabilities

- `worktree-list`: Lists all bonsai-managed worktrees with their path and
  availability status, using ANSI colour output.

### Modified Capabilities

<!-- No existing spec-level requirements are changing. -->

## Impact

- `src/cli.rs` or equivalent: new `list` / `ls` subcommand wired into the CLI.
- `src/commands/` (or equivalent module): new `list.rs` command handler.
- Calls `git worktree list --porcelain` (or the equivalent libgit2 API) to
  enumerate worktrees.
- No breaking changes to existing subcommands.
- New dependency on a terminal-colour crate (e.g. `colored` or `owo-colors`) if
  not already present.
