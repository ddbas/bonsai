## Context

Bonsai manages a pool of git worktrees at `~/.bonsai/<repo-slug>/`. Each slot is
an 8-char UUID directory registered with git via `git worktree add`. Git already
maintains a per-worktree lock flag (stored in `.git/worktrees/<id>/locked`)
toggled by `git worktree lock` and `git worktree unlock`. Bonsai already reads
this flag via `git worktree list --porcelain` and marks locked slots as `InUse`,
preventing them from being reused by `bs get`. The gap is that users currently
have no bonsai command to set that flag themselves.

## Goals / Non-Goals

**Goals:**

- Expose `bs lock` and `bs unlock` as thin, idiomatic wrappers around
  `git worktree lock` and `git worktree unlock`.
- Accept either an 8-char slug (e.g. `a3f9c1b2`) or an absolute/tilde path as
  the target argument.
- Default to the current slot (CWD) when no argument is given.
- Forward an optional `--reason <string>` to git on `lock`.
- Reject targets that are not bonsai-managed pool slots.

**Non-Goals:**

- Inventing a new lock file or database — git's built-in mechanism is the source
  of truth.
- Supporting cross-repo lock operations.
- Any UI for listing lock reasons (readable via `git worktree list --porcelain`
  which shows the `locked` annotation).

## Decisions

### 1 — Delegate entirely to `git worktree lock / unlock`

**Decision**: Shell out to `git worktree lock [--reason <msg>] <path>` and
`git worktree unlock <path>`. Do not touch `.git/worktrees/*/locked` directly.

**Rationale**: Using the official porcelain keeps bonsai consistent with git's
own state and ensures the lock is visible to other git tooling. Direct file
manipulation would be fragile across git versions.

**Alternative considered**: Write/delete `.git/worktrees/<id>/locked` directly.
Rejected because it bypasses git's internal state management and could break if
git changes the format.

### 2 — Slug or path resolution

**Decision**: Add a
`resolve_slot_path(input: &str, pool_dir: &Path) -> Result<PathBuf>` helper. It
checks whether `input` is:

1. An absolute or tilde-expanded path that falls under `pool_dir` → use directly
   after canonicalization.
2. An 8-char hex string matching an existing slot directory name → resolve to
   `pool_dir/<slug>`.
3. Anything else → error: "not a bonsai-managed slot".

**Rationale**: Users naturally think of slots by their short UUID. Allowing an
absolute path also means `bs lock $(bs get)` just works.

### 3 — Default to current slot

**Decision**: When no positional argument is supplied, call `current_worktree()`
and error if the CWD is not inside a managed slot.

**Rationale**: The most common use case is "I'm in a slot right now and I want
to lock it". Requiring the path explicitly would add friction.

### 4 — Error on non-pool paths

**Decision**: After slug/path resolution, verify the resolved path is a
registered bonsai pool slot before calling git. Produce a clear error message
that names the pool directory.

**Rationale**: Silently passing an arbitrary path to `git worktree lock` could
lock a slot in a different repo's pool, which would confuse users. Validation
keeps bonsai's scope clear.

## Risks / Trade-offs

- **git version compatibility**: `git worktree lock --reason` was added in git
  2.17 (April 2018). Bonsai already requires git for all operations so this is
  an acceptable baseline. → _Mitigation_: document the minimum git version in
  README; git itself will produce an informative error on older versions.

- **Double-locking**: Calling `bs lock` on an already-locked slot errors from
  git (`fatal: worktree is already locked`). Bonsai will surface this error
  unchanged — it is accurate and actionable. → _Mitigation_: no special handling
  needed; the error message is clear.

- **Race condition**: Between checking that a slot is a managed pool slot and
  calling git, another process could remove the slot. → _Mitigation_: git will
  error with "not a worktree" — acceptable.
