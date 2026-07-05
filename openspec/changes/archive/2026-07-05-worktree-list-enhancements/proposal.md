## Why

The `bs list` output currently shows a status badge and path, but omits two
pieces of information that are essential for at-a-glance pool inspection: which
branch (if any) a worktree has checked out, and a breakdown of _why_ a slot is
in use (processes, dirty files, or both). Users must run additional git commands
to understand whether a worktree is safe to reclaim.

## What Changes

- **Branch column in the path column**: Each line will show the checked-out
  branch name (or nothing for detached HEAD) in **bold parentheses** immediately
  after the tilde-abbreviated path, e.g. `~/.bonsai/bonsai/a3f9c1b2 **(main)**`.
- **Expanded stats column**: The third column is replaced with a compact,
  icon-driven summary of all stats that contribute to a slot being "in use":
  process count (`⚙N`), uncommitted/modified+staged file count (`±N`), and
  untracked file count (`?N`). Only non-zero values are shown, so clean slots
  display nothing (or a minimal indicator).

## Capabilities

### New Capabilities

- `worktree-branch-display`: Display the checked-out branch (or detached HEAD
  indicator) of each pool worktree in the list output, shown in bold parentheses
  after the path.
- `worktree-usage-stats`: Replace the single process-count column with a compact
  multi-stat column showing open-process count (`⚙`), uncommitted-file count
  (`±`), and untracked-file count (`?`), each only rendered when non-zero.

### Modified Capabilities

- `worktree-list`: Update output format — the path column gains a bold branch
  suffix, and the third column changes from a raw process count to the new
  icon-based stats summary.

## Impact

- `src/worktree/mod.rs`: New helper functions to read the current branch from
  `git worktree list --porcelain` (already available as `branch` field) and to
  count uncommitted/untracked files via `git status --porcelain`.
- `src/main.rs`: Updated rendering loop to emit branch name and multi-stat
  column.
- `openspec/specs/worktree-list/spec.md`: Scenarios and requirements updated for
  new column format.
- No new external dependencies; uses existing `git` and `owo-colors`
  infrastructure.
