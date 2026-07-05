## Context

`bs get` and `bs list` classify every pool slot as either **available** or **in
use** before deciding whether a slot can be handed to a new task. The current
classification uses three signals: the slot directory must exist on disk, it
must not be git-locked, and its working tree must be clean (no uncommitted
changes).

A slot that passes all three tests can still be actively used by another process
— an editor, a compiler daemon, or a shell that was `cd`-ed into it. Handing
that slot to a new task while a process still holds file descriptors inside the
directory can cause silent data corruption, confusing build failures, or lost
work.

The fix is a fourth availability precondition: no process may have any file in
the slot directory open. The standard Unix tool for this query is `lsof`.

## Goals / Non-Goals

**Goals:**

- Add a `has_open_files(path)` helper that returns `true` when `lsof` reports
  any open file descriptor under `path`.
- Integrate the check into `find_available_slot` and `list_worktrees_status` so
  that a slot with open file descriptors is treated as `WorktreeStatus::InUse`.
- Fail fast with a clear, actionable error message when `lsof` is not found on
  `PATH`, so the user knows exactly which dependency is missing rather than
  silently getting degraded behaviour.
- Keep the implementation in `src/worktree/mod.rs`; no new crate dependencies.

**Non-Goals:**

- Cross-platform support beyond macOS and Linux (Windows support deferred).
- Tracking _which_ process holds the handle or displaying that information to
  the user.
- Modifying the git-lock mechanism or the dirty-tree check.

## Decisions

### Decision: Use `lsof +D <path>` rather than `/proc` scanning

`lsof +D <path>` recursively lists all open files under a directory on both
macOS and most Linux distributions. It is a single well-understood command.

Alternatives considered:

- **Parse `/proc/<pid>/fd/`** — Linux-only, requires root for other users'
  processes, more complex to implement.
- **`fuser -m <path>`** — mounts-based, not recursive for directories,
  inconsistent flags across distributions.
- **`inotify` / `kqueue`** — event-driven, not a point-in-time query.

`lsof +D` is the clear winner for portability and simplicity.

### Decision: Non-zero exit code from `lsof` means "no open files" (not an error)

`lsof` exits with code 1 when it finds no matching files, which is the common
case for available slots. An actual error (binary missing, permission denied)
also returns non-zero but typically produces stderr output. We distinguish these
by:

1. Exit 0 with any stdout → open files exist → `has_open_files` returns `true`.
2. Exit 1 with empty stdout → no open files → `has_open_files` returns `false`.
3. `lsof` not found (spawn error) → propagate as a hard error with a clear
   message.
4. Exit non-zero with non-empty stderr → return `Err(...)` → caller propagates
   the error.

### Decision: Fail fast — `lsof` absent ⇒ hard CLI error

If `lsof` cannot be spawned (not installed, not on `PATH`), `has_open_files`
returns an `Err` with a message of the form:

> `lsof not found on PATH — install lsof to use bs (e.g. brew install lsof)`

The callers in `find_available_slot`, `list_worktrees_status`, and
`get_worktree` propagate the error up to `main`, which prints it and exits with
a non-zero status.

Alternative considered: silently treat the slot as InUse. Rejected because it
hides a missing system dependency behind confusing behaviour ("why does `bs get`
always create a new slot?") and makes the system harder to reason about.

### Decision: Run `lsof` with a short timeout via `std::process::Command`

No async runtime is needed for this synchronous helper. A tight timeout is not
strictly required because `lsof +D` on a small directory completes in
milliseconds, but if performance becomes a concern in the future the check can
be made async or cached.

## Risks / Trade-offs

- **`lsof` not installed** → CLI exits with a non-zero status and an actionable
  error message. _Mitigation_: Error message includes install instructions (e.g.
  `brew install lsof`). Document `lsof` as a required runtime dependency in
  README.
- **`lsof` performance on large trees** → `lsof +D` can be slow if the slot has
  many files. _Mitigation_: Pool slots are generally small checkout trees;
  acceptable for now. Future optimisation: use `lsof` file-descriptor limit
  flags.
- **Race condition** → A process could open a file between the `lsof` call and
  the actual slot checkout. _Mitigation_: This race is inherent to all open-file
  detection schemes and is acceptable — the window is tiny and the consequence
  is a build failure, not data loss.
- **macOS SIP / sandboxing** → `lsof` may not see all file descriptors of
  sandboxed processes. _Mitigation_: Sandboxed apps typically don't use raw file
  paths from the bonsai pool; risk is low.

## Migration Plan

- No stored state changes; no migration required.
- The new check is additive: slots that were previously `Available` will remain
  `Available` unless they have open file handles. Existing behaviour is fully
  preserved for the common case.
- Rollback: revert the `has_open_files` call sites; no other artefacts to undo.

## Open Questions

- Should the error message for `lsof` not found include platform-specific
  install commands (e.g. `apt install lsof` on Debian vs `brew install lsof` on
  macOS)? For now a generic message is sufficient; platform detection can be
  added later.
