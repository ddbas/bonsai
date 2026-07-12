## Why

Bonsai pool slots can be temporarily reserved for exclusive use (e.g. by an
agent or a long-running process), but there is currently no way to express that
intent explicitly. Without a `lock`/`unlock` command, users must rely on
implicit "in use" detection (open files, dirty state) which cannot capture
intentional reservation. Git already provides `git worktree lock` and
`git worktree unlock` with an optional human-readable reason — bonsai should
expose those mechanics directly instead of inventing a parallel system.

Additionally, the `bs list` display currently collapses "locked" and "in use"
into a single red `in use` badge, making it impossible to distinguish an
intentionally reserved slot from one that simply has open files or uncommitted
changes. A dedicated `locked` status (yellow) makes pool state self-explanatory
at a glance.

## What Changes

- **New `bs lock [--reason <message>] [<path>]`** subcommand: locks the target
  pool slot via `git worktree lock`, preventing `bs get` from reusing it.
  Defaults to the current slot when inside a managed worktree; accepts an
  explicit absolute path when provided. Accepts an optional `--reason` string
  forwarded verbatim to git.
- **New `bs unlock [<path>]`** subcommand: unlocks the target pool slot via
  `git worktree unlock`. Defaults to the current slot when no argument is given;
  accepts an explicit absolute path when provided.
- **No slug support** — both commands accept a full absolute path (easily copied
  from `bs list` output) or no argument (default to current slot).
- Attempting to lock/unlock a path that is not a bonsai-managed slot is an
  error.
- **`WorktreeStatus` gains a `Locked` variant** separate from `InUse`. A slot
  that is git-locked is always displayed as `locked` regardless of whether it
  also has open processes or uncommitted changes (`Locked` takes priority over
  `InUse`).
- **`bs list` shows `locked` in yellow** with the label `locked`. Usage-stats
  icons (`⚙N`, `±N`, `?N`) continue to appear on locked rows when non-zero.
- The `TODO.md` `lock & unlock` item is removed.

## Capabilities

### New Capabilities

- `worktree-lock`: Expose `bs lock` — locks a bonsai pool slot using
  `git worktree lock`, optionally recording a human-readable reason. Defaults to
  the current slot; accepts a full path argument.
- `worktree-unlock`: Expose `bs unlock` — unlocks a bonsai pool slot using
  `git worktree unlock`. Defaults to the current slot; accepts a full path
  argument.

### Modified Capabilities

- `worktree-list`: The slot display gains a third status badge — yellow `locked`
  — for git-locked slots. `WorktreeStatus` in `worktree-get`'s pool scanning
  logic also gains the `Locked` variant so that locked slots are never handed
  out by `bs get`.

## Impact

- `src/main.rs`: add `Lock` and `Unlock` variants to `Commands`; dispatch arms
  resolve the optional path argument or fall back to `current_worktree()`.
- `src/worktree/mod.rs`:
  - `WorktreeStatus`: add `Locked` variant.
  - `list_worktrees_status`: classify locked slots as `WorktreeStatus::Locked`
    (regardless of open-file / dirty state).
  - Add `lock_worktree(path: &Path, reason: Option<&str>) -> Result<()>`.
  - Add `unlock_worktree(path: &Path) -> Result<()>`.
  - Add `validate_pool_slot(path: &Path, pool_dir: &Path) -> Result<()>` for
    pre-call validation.
  - Remove `resolve_slot_path` slug helper (no longer needed).
- No new external dependencies.
- No breaking changes to existing subcommands.
