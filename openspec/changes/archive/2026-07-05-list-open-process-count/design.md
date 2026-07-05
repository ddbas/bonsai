## Context

`bs list` iterates `list_worktrees_status`, which today returns
`Vec<(PathBuf, WorktreeStatus)>`. The companion change
(`worktree-in-use-open-files`) adds `has_open_files` (boolean) and integrates it
into that function. This change goes one step further: we need a **count** of
distinct processes, not just a yes/no, so the list can display it.

The only output surface is `src/main.rs` — the `Commands::List` match arm that
prints one line per slot. Columns are currently hand-padded with spaces; there
is no table formatter.

## Goals / Non-Goals

**Goals:**

- Add `count_open_processes(path) -> Result<usize>` that parses unique PIDs from
  `lsof +D <path>` output.
- Extend `list_worktrees_status` to return an `Option<usize>` process count
  alongside each `(PathBuf, WorktreeStatus)`.
- Render the count as a right-aligned column in `bs list`; leave it blank for
  `Available` slots and for `InUse` slots with no open file handles (locked or
  dirty-but-idle). A dirty slot that also has open file handles shows the count.

**Non-Goals:**

- Listing which processes are holding the slot open (just the count).
- A full table-layout library; simple column padding is sufficient.
- Changing how `has_open_files` / `find_available_slot` work — they stay
  boolean.

## Decisions

### Decision: Reuse a single `lsof +D` call — parse PIDs from the same output

`lsof +D <path>` outputs one row per open file descriptor. Each row's second
field is the PID. Counting unique values in that column gives the process count
with no additional syscall. We parse this in `count_open_processes` and also
expose a combined `open_processes(path) -> Result<usize>` that returns 0 when
`lsof` finds nothing (exit 1, empty stdout).

`list_worktrees_status` can then call `count_open_processes` once per slot and
derive both the boolean "has open files" check and the count from the result
(`count > 0` replaces the `has_open_files` call).

Alternatives considered:

- **Keep `has_open_files` separate, add a second `lsof` call for the count** —
  doubles the `lsof` invocations per slot. Rejected.
- **Store raw lsof output and pass it around** — couples layers unnecessarily.
  Rejected.

### Decision: `list_worktrees_status` return type changes to `Vec<(PathBuf, WorktreeStatus, Option<usize>)>`

`Option<usize>` is `Some(n)` when `n > 0` open processes were detected
(regardless of whether the working tree is also dirty), and `None` in every
other case (available, locked, or lsof error-propagated). A dirty slot with open
file handles is `Some(n)` — the process count is the primary signal. This keeps
the rendering layer simple: render the column only when the value is `Some`.

Alternative: a richer `SlotInfo` struct. Deferred — three-tuple is fine until
more fields are needed.

### Decision: Column is blank (not "0") for slots with no open processes

Showing "0" for a locked or dirty (but idle) slot would be misleading — it
implies the open-file check ran and found nothing, when in fact no processes are
holding the slot. Blank is unambiguous. `Some(0)` cannot occur by construction
(we set `None` whenever the count is zero), but even if it did, we'd still
render blank.

### Decision: Simple fixed-width padding, no table-layout crate

The list has exactly two data columns (badge + path) plus the optional count. A
`format!("{:>N}", count)` right-pad is sufficient. Adding a crate for this would
be over-engineering.

## Risks / Trade-offs

- **Return-type change is a breaking API change inside the crate** → Both call
  sites (`main.rs` and tests) must be updated. Low risk since this is an
  internal crate with no external consumers.
- **`lsof` parse fragility** → PID is the second whitespace-delimited field on
  each data line (after the command name). This format is stable across macOS
  and Linux `lsof` versions. A header line starting with "COMMAND" is skipped.
- **Race** → Process count is a snapshot; a process could exit between `lsof`
  and display. Acceptable — same caveat applies to the boolean check.

## Migration Plan

No stored state or external API changes. Existing tests that assert on
`list_worktrees_status`'s return type need the new third tuple element; update
them in the same PR.

## Open Questions

_(none)_
