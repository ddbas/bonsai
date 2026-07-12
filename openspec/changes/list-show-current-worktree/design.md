## Context

`bs list` displays all managed pool worktrees with colour-coded availability
badges, tilde-abbreviated paths, branch names, and usage stats. The
recently-shipped `bs current` subcommand uses `worktree::current_worktree()` to
compare the process CWD against registered pool slots and return the matching
slot (if any).

Currently the two commands are disconnected: `bs list` has no awareness of which
slot the user is presently inside. Adding that awareness requires a single
additional call to `current_worktree()` before the render loop, followed by a
per-row check.

## Goals / Non-Goals

**Goals:**

- Visually distinguish the active slot in `bs list` output so users can orient
  themselves at a glance.
- Reuse the existing `current_worktree()` function — no new detection logic.
- Keep the change minimal and isolated to the `Commands::List` render path in
  `src/main.rs`.

**Non-Goals:**

- Changing the output format for the `bs current` subcommand.
- Adding machine-readable (JSON/porcelain) output to `bs list`.
- Any changes to the availability/status classification logic.

## Decisions

### Decision: Prefix the current row with `▶` and append `(current)` label

**Options considered:**

1. **Prefix `▶` only** — Stands out immediately but gives no textual hint for
   colour-blind or plain-text users.
2. **`(current)` label only** — Text is clear but rows look identical until the
   label is spotted at the end.
3. **`▶` prefix + `(current)` label after branch** — Combines spatial (first
   column) and semantic cues. Chosen because it stays legible in plain-text
   environments (e.g. `| cat` piped output) and matches conventions used by
   tools like `git branch` (which uses `*`).

The `▶` indicator goes at the very start of the line, replacing the leading
spaces that currently pad the status column area, so column alignment is
preserved.

### Decision: Call `current_worktree()` once, tolerate `Err` gracefully

`current_worktree()` can fail if `git` is not available or if CWD resolution
fails. Rather than propagating that error and breaking `bs list`, we treat any
error as "no current slot detected" (`None`). This keeps the list command
maximally useful even in degraded environments.

### Decision: Path comparison via canonical PathBuf equality

`current_worktree()` already returns a canonicalised `PathBuf`. The `entries`
from `list_worktrees_status` also use canonical paths (they are resolved in
`list_pool_worktrees`). A direct `==` comparison on `PathBuf` is therefore
reliable without additional string manipulation.

## Risks / Trade-offs

- **`▶` glyph rendering**: In some legacy terminals the `▶` character may render
  incorrectly. Mitigation: the `(current)` text label provides a fallback signal
  that is always legible in ASCII.
- **One extra subprocess call**: `current_worktree()` invokes `git rev-parse`
  and `git worktree list --porcelain`. These are fast, but they add latency.
  Mitigation: call occurs once before the render loop, not per row.
