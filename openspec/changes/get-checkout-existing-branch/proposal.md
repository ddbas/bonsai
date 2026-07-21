## Why

Today `bs get` only checks out a slot at detached HEAD, or creates a
**new**/reset branch via `-b`/`-B`. There is no way to grab a worktree for a
branch that already exists elsewhere in the repo (e.g. a teammate's feature
branch, or a branch you created in a previous session) without manually running
`git checkout <branch>` inside the slot afterward. Adding a positional branch
argument lets `bs get <branch>` provision a slot and check out that existing
branch in one step, matching the ergonomics of
`git worktree add <path> <branch>`.

## What Changes

- Add an optional positional `branch` argument to `bs get`:
  `bs get <branch-name>` provisions (or reuses) a slot exactly as `bs get` does
  today, then checks out the **existing** branch `<branch-name>` inside it (via
  `git checkout <branch-name>`, not `-b`/`-B`).
- If `<branch-name>` does not exist in the repository, the command SHALL exit
  with a non-zero status and an actionable error message (it SHALL NOT create
  the branch).
- The positional `branch` argument and the `-b`/`-B` flags are mutually
  exclusive; supplying both SHALL result in a non-zero exit and a usage error.
- If a worktree already exists for the given branch (i.e. the branch is
  currently checked out in another worktree, managed or not),
  `bs get <branch-name>` SHALL error out, mirroring `git worktree add`'s own
  "already checked out" behavior, instead of silently reusing or resetting a
  slot.
- Output continues to show the branch name alongside the slot path, matching
  existing `-b`/`-B` behavior.

## Capabilities

### New Capabilities

- `get-checkout-existing-branch`: `bs get <branch>` provisions a worktree slot
  and checks out an existing branch into it, erroring if the branch doesn't
  exist or is already checked out elsewhere.

### Modified Capabilities

- `get-branch-options`: the mutual-exclusion requirement between `-b` and `-B`
  is extended to also cover the new positional `branch` argument (all three
  forms — positional branch, `-b`, `-B` — are pairwise mutually exclusive).

## Impact

- Affected code: `src/main.rs` (CLI argument parsing, `Commands::Get`),
  `src/worktree/mod.rs` (`get_worktree`, `BranchMode`, slot reset/create logic,
  branch-existence and already-checked-out checks).
- No changes to on-disk pool layout, locking, or the `-b`/`-B` semantics already
  implemented.
