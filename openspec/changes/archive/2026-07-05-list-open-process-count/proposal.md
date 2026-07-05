## Why

`bs list` currently tells you a slot is "in use" but not _how_ it is in use.
When open file handles are what's keeping a slot occupied (introduced in
`worktree-in-use-open-files`), the user has no quick way to gauge how many
processes are holding it. A process-count column gives instant insight — one
glance distinguishes a single idle editor from a full build farm, making it
easier to decide whether to wait or force-provision a new slot.

## What Changes

- Add a `count_open_processes(path) -> Result<usize>` helper to the `worktree`
  module that returns the number of **distinct PIDs** with open file handles
  inside a slot directory (parsed from `lsof +D <path>` output).
- Extend `list_worktrees_status` to return the process count alongside the
  existing status, so the rendering layer has the number without a second `lsof`
  call.
- Update the `bs list` output to include a right-aligned process-count column.
  The column value is printed only when the count is ≥ 1; for available slots
  the column is left blank so it does not clutter clean output.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `worktree-list`: display format gains a process-count column (blank for
  available slots, numeric for in-use slots with open handles).
- `worktree-open-file-detection`: gains `count_open_processes` function that
  counts distinct PIDs from `lsof +D` output.

## Impact

- **`src/worktree/mod.rs`**: new `count_open_processes(path)` function; updated
  `list_worktrees_status` return type (adds `Option<usize>` process count per
  entry).
- **`src/main.rs`**: updated `List` rendering to consume the count and format
  the new column.
- **No new crate dependencies** — `lsof` is already a required runtime
  dependency.
- **Tests**: unit tests for `count_open_processes`; updated snapshot/unit tests
  for the list output format.
