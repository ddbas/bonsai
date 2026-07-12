## Why

Bonsai pool slots can be temporarily reserved for exclusive use (e.g. by an
agent or a long-running process), but there is currently no way to express that
intent explicitly. Without a `lock`/`unlock` command, users must rely on
implicit "in use" detection (open files, dirty state) which cannot capture
intentional reservation. Git already provides `git worktree lock` and
`git worktree unlock` with an optional human-readable reason — bonsai should
expose those mechanics directly instead of inventing a parallel system.

## What Changes

- **New `bs lock [--reason <message>] [<worktree>]`** subcommand: locks the
  target pool slot via `git worktree lock`, preventing `bs get` from reusing it.
  Defaults to the current slot when no path/slug is given. Accepts an optional
  `--reason` string forwarded verbatim to git.
- **New `bs unlock [<worktree>]`** subcommand: unlocks the target pool slot via
  `git worktree unlock`. Defaults to the current slot when no argument is given.
- Both commands accept either an absolute path or the 8-char UUID slug that
  identifies a slot (e.g. `a3f9c1b2`).
- Attempting to lock/unlock a path that is not a bonsai-managed slot is an
  error.
- The `TODO.md` item "Show current worktree in `list` subcommand" is complete
  and has been removed.

## Capabilities

### New Capabilities

- `worktree-lock`: Expose `bs lock` — locks a bonsai pool slot using
  `git worktree lock`, optionally recording a human-readable reason.
- `worktree-unlock`: Expose `bs unlock` — unlocks a bonsai pool slot using
  `git worktree unlock`.

### Modified Capabilities

_(none — existing spec-level behaviour is unchanged)_

## Impact

- `src/main.rs`: add `Lock` and `Unlock` variants to the `Commands` enum and
  their dispatch arms.
- `src/worktree/mod.rs`: add `lock_worktree(path, reason)` and
  `unlock_worktree(path)` public functions that shell out to git; add
  `resolve_slot_path(slug_or_path)` helper for slug→`PathBuf` resolution.
- No new dependencies required (git is already the only external process).
- No breaking changes to existing subcommands.
