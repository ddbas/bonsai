## Why

Spinning up a new git worktree for every task is expensive — it requires
re-running setup steps (e.g. `npm install`, build tool caching) on each fresh
worktree. A `get` command that provisions and reuses worktrees from a managed
pool under `~/.bonsai/` amortises that cost, making ephemeral task environments
cheap to acquire.

## What Changes

- Add a `get` subcommand to the `bs` CLI that creates or reuses a managed git
  worktree.
- Make `get` the **default** command: running `bs` without a subcommand is
  equivalent to `bs get`.
- Define a canonical worktree storage layout under `~/.bonsai/<repo>/<slot>/` so
  multiple projects can coexist.
- Introduce a **pool model**: UUID-named slots are provisioned once and reused
  when idle, rather than created or named per-branch.
- All managed worktrees use **detached HEAD** — the user is responsible for
  creating/checking out branches inside the worktree.
- `get` correctly resolves HEAD and the repo root whether it is called from the
  main worktree or from a linked worktree.
- Remove `arg_required_else_help` gate so the default command can fire without a
  subcommand.

## Capabilities

### New Capabilities

- `worktree-get`: Core `bs get` command — resolves the calling worktree's HEAD,
  scans the pool for an available (clean, unlocked) slot, resets it to that HEAD
  (detached) or creates a new UUID-named slot if none are free, and prints the
  path to stdout.

### Modified Capabilities

- (none — no existing spec requirements are changing)

## Impact

- `src/main.rs`: new `Get` variant in `Commands`, default-command dispatch
  logic.
- New source file(s) for worktree management logic (e.g. `src/worktree/`).
- Runtime: reads/writes `~/.bonsai/<repo>/` directory; shells out to
  `git worktree` commands.
- No new external Cargo dependencies expected for the initial scaffold (git
  operations via `std::process::Command`); `dirs` crate added for cross-platform
  home-dir resolution; `uuid` crate for slot name generation.
