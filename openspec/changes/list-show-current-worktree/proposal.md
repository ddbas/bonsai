## Why

The `bs current` subcommand was added to tell users which managed worktree slot
they are presently inside, but `bs list` still shows every slot without any
indication of which one is active. Users must mentally cross-reference the two
commands to understand their context at a glance.

## What Changes

- The `bs list` output will detect the current worktree (reusing the logic
  introduced for `bs current`) and visually distinguish the matching row with a
  `▶` indicator prefix and `(current)` label so the active slot stands out
  immediately.
- The `current` subcommand TODO checkbox is removed from `TODO.md` as it has
  already been implemented.

## Capabilities

### New Capabilities

_(none — all changes extend an existing capability)_

### Modified Capabilities

- `worktree-list`: add a requirement that the list output marks the
  currently-active pool slot (if any) with a visual indicator, reusing the same
  CWD-detection logic as `bs current`.

## Impact

- `src/main.rs` — `Commands::List` arm: call `worktree::current_worktree()` once
  before the render loop and annotate the matching row.
- `openspec/specs/worktree-list/spec.md` — new requirement + scenarios for the
  current-slot indicator.
- `TODO.md` — remove the `[ ] current subcommand` checkbox.
- No breaking changes; no new subcommands; no public API additions.
