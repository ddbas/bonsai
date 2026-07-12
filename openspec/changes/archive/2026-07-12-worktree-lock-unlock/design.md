## Context

Bonsai manages a pool of git worktrees at `~/.bonsai/<repo-slug>/`. Each slot is
an 8-char UUID directory registered with git via `git worktree add`. Git already
maintains a per-worktree lock flag (stored in `.git/worktrees/<id>/locked`)
toggled by `git worktree lock` and `git worktree unlock`. Bonsai already reads
this flag via `git worktree list --porcelain` and currently marks locked slots
as `InUse`, preventing them from being reused by `bs get`. The gaps are:

1. Users have no bonsai command to set the lock flag themselves.
2. The `bs list` display collapses locked and in-use into a single red badge,
   hiding intent from the user.

## Goals / Non-Goals

**Goals:**

- Expose `bs lock` and `bs unlock` as thin, idiomatic wrappers around
  `git worktree lock` and `git worktree unlock`.
- Accept an absolute path argument, defaulting to the current slot (CWD) when no
  argument is provided.
- Forward an optional `--reason <string>` to git on `lock`.
- Reject targets that are not bonsai-managed pool slots.
- Introduce `WorktreeStatus::Locked` as a first-class variant distinct from
  `InUse`; display it as a yellow `locked` badge in `bs list` with stats icons
  still present.
- When a slot is both git-locked and has in-use signals (open files, dirty
  working tree), classify it as `Locked` (not `InUse`).

**Non-Goals:**

- Slug-based slot addressing (`bs lock a3f9c1b2`). The full path from `bs list`
  is sufficient and unambiguous.
- Inventing a new lock file or database — git's built-in mechanism is the source
  of truth.
- Supporting cross-repo lock operations.
- Displaying the lock reason in `bs list` output (it is accessible via
  `git worktree list --porcelain`).

## Decisions

### 1 — Delegate entirely to `git worktree lock / unlock`

**Decision**: Shell out to `git worktree lock [--reason <msg>] <path>` and
`git worktree unlock <path>`. Do not touch `.git/worktrees/*/locked` directly.

**Rationale**: Using the official porcelain keeps bonsai consistent with git's
own state and ensures the lock is visible to other git tooling. Direct file
manipulation would be fragile across git versions.

**Alternative considered**: Write/delete `.git/worktrees/<id>/locked` directly.
Rejected: bypasses git's internal state management.

### 2 — Full path only (no slug resolution)

**Decision**: The optional positional argument for `bs lock` and `bs unlock` is
an absolute path. No 8-char slug shorthand is supported.

**Rationale**: Users already see full tilde-prefixed paths in `bs list` output
and can copy-paste them. Adding slug resolution would complicate the code
(requires scanning the pool directory) with minimal benefit; full paths are
unambiguous and transparent.

**Alternative considered**: Accept an 8-char UUID slug (`a3f9c1b2`). Rejected:
extra complexity, tilde paths from `bs list` are already convenient.

### 3 — Default to current slot

**Decision**: When no positional argument is supplied, call `current_worktree()`
and error if the CWD is not inside a managed slot.

**Rationale**: The most common use case is "I'm in a slot right now and I want
to lock it." Requiring an explicit path would add friction for the common case.

### 4 — `Locked` takes priority over `InUse`

**Decision**: In `list_worktrees_status`, check the `locked` flag first. If a
slot is locked, return `WorktreeStatus::Locked` unconditionally — do not inspect
open-process or dirty-state signals for classification purposes. Stats
(`process_count`, `uncommitted_count`, `untracked_count`) are still collected
and included in the returned `WorktreeStats` so they can be shown in the list
output.

**Rationale**: A locked slot is intentionally reserved; showing it as `in use`
would obscure that intent. Users who lock a slot with a dirty tree or open
editors still need to see those stats for awareness, but the primary badge
should reflect the deliberate reservation.

**Alternative considered**: Compute both and let the display layer pick.
Rejected: classification should be authoritative at the model layer, not
presentation logic.

### 5 — Yellow `locked` badge in `bs list`

**Decision**: Display `WorktreeStatus::Locked` as a yellow `locked` label.
Continue showing stats icons on locked rows for the same icons that appear on
`in use` rows.

**Rationale**: Yellow communicates "reserved / attention needed" without the
urgency of red. Stats icons remain useful so users can still see whether a
locked slot is also dirty or active.

## Risks / Trade-offs

- **git version compatibility**: `git worktree lock --reason` was added in git
  2.17 (April 2018). Bonsai already requires git for all operations; git will
  produce its own error on older versions. → _Mitigation_: document minimum git
  version in README.

- **Double-locking / double-unlocking**: Git errors with
  `fatal: worktree is already locked` / `fatal: worktree is not locked`. Bonsai
  surfaces these errors unchanged. → _Mitigation_: errors are clear and
  actionable; no special handling needed.

- **Race condition**: Between validating that a path is a pool slot and calling
  git, another process could remove the slot. Git will error with "not a
  worktree". → _Mitigation_: acceptable; the error is clear.
