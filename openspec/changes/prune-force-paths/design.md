## Context

`prunePool` (`packages/core/src/pool.ts`) classifies slots via
`listWorktreesStatus`, preserves the first `available` slot (detaching its
branch via `resetSlot` → `git checkout --detach`) and deletes the rest.
Classification is `locked` > `in use` > `available` (`classify.ts`). A slot is
`available` iff: unlocked, `git status` shows no uncommitted and no untracked
files, and its branch state shows nothing unmerged (detached at, or branch
merged into, the default ref). Git-ignored files are not considered.

## Goals / Non-Goals

**Goals:**

- Candidate narrowing by path; `--force` for `in use` slots, with or without
  paths.
- Preservation always applies; a forced preserved slot is sanitized to
  `available`.
- Locked slots are unprunable and unmodifiable by construction.

**Non-Goals:**

- Overriding locks, deleting branches, removing git-ignored files, editing git
  bookkeeping (still delegated to `git worktree prune`).

## Decisions

- **Extend `prunePool(deps, { paths, force })`** rather than adding a parallel
  use case: the preserve/delete/`worktree prune`/failure flow is identical; only
  candidate selection, preserved-slot selection, and sanitizing differ.
- **Plan then act.** Build a plan (candidates, preserved slot, deletions) from
  classification, validating path targets first (unmanaged / locked / in use
  without force → one aggregated `BonsaiError`, nothing touched). Rationale: a
  destructive command shouldn't partially execute on a typo.
- **Locked is filtered out of the candidate set before `force` is consulted**;
  `force` has no code path that reaches a locked slot. Explicit locked target →
  error; implicit (no paths) → skipped.
- **Preserved-slot order:** untargeted available > first available candidate >
  (force) first in-use candidate. Prefers the choice that discards nothing.
- **Sanitize = what makes a slot `available`:** `git reset --hard` (clears
  staged/unstaged and merge state), `git clean -fd` (untracked; NOT `-x`, so
  ignored files such as `bonsai.copy` files survive), then detach at repo HEAD
  via existing `resetSlot`. Detached at the repo HEAD commit is "merged", so the
  branch-state check passes. Needs a new git port method (e.g. `resetHard` +
  `clean`, or a single `sanitize(slot)`), implemented in `packages/git`.
- **Branches are never deleted.** Detaching drops the slot's checkout only; the
  ref (and its commits) remain.
- `--force` with no paths applies to every unlocked `in use` slot. Reporting and
  failure handling reuse the existing output.

## Risks / Trade-offs

- Force discards uncommitted/untracked work irreversibly → opt-in flag only;
  locks remain the protection mechanism.
- Detached-HEAD slots with their own commits lose the only branch-less
  reference to them (reachable via reflog only) → documented; no mitigation.
- A missing-directory (stale) slot classifies as `in use`; deletion is a no-op
  and `git worktree prune` deregisters it. A stale slot must never be chosen for
  sanitizing (skip when the directory does not exist).
- In-progress rebase/cherry-pick state under `.git/worktrees/<name>` is not
  cleared by reset/clean; it doesn't affect classification, so it is out of
  scope for now.
- Pruning the slot containing the cwd leaves the shell in a deleted directory →
  accepted.

## Open Questions

- Should `--force` also remove git-ignored files from a sanitized slot (`clean
  -x`)? Default here is no.
