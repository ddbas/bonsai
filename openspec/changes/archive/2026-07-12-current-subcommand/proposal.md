## Why

When working across multiple worktrees, users need a quick way to identify which
managed pool slot they are currently inside — without running `bs list` and
visually scanning the output. The `current` subcommand fills this gap by
printing the active slot's path (and branch if set) in a single, scriptable
command.

## What Changes

- Add a `bs current` subcommand that detects whether the process CWD is inside a
  managed bonsai pool slot and prints its path.
- If the CWD is inside a known pool slot, output the tilde-abbreviated path (and
  branch name when on a named branch), mirroring the `bs list` row format.
- If the CWD is **not** inside any managed slot, print a human-readable message
  and exit with a non-zero status code so callers can detect the condition
  programmatically.

## Capabilities

### New Capabilities

- `worktree-current`: Detect and display the managed bonsai worktree slot that
  contains the current working directory, including its tilde-abbreviated path
  and checked-out branch name.

### Modified Capabilities

_(none)_

## Impact

- `src/main.rs`: adds the `Current` variant to the `Commands` enum and its match
  arm.
- `src/worktree/mod.rs`: adds a `current_worktree` public function that resolves
  the CWD against the managed pool.
- No new dependencies required.
- No breaking changes to existing subcommands or public API.
