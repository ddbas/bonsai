## Why

Once bonsai has file-based logging (see `add-structured-logging`), users and
support requests will need a quick way to discover where bonsai keeps its files
without reading source code or guessing platform conventions. There is currently
no single command that surfaces bonsai's own install/runtime locations.

## What Changes

- Add a new `bs info` subcommand that prints bonsai's own runtime paths to
  stdout in a simple, human-readable, script-friendly format.
- Initial output includes:
  - the log directory path (as resolved by the logging subsystem introduced in
    `add-structured-logging`)
  - the bonsai managed root path (`~/.bonsai`, i.e. the parent of all per-repo
    worktree pools)
  - the bonsai version (`CARGO_PKG_VERSION`)
  - the effective log level (the resolved default or `--log-level` override for
    this invocation)
  - the path to the current day's active log file
- `bs info` depends on the logging subsystem (`add-structured-logging`) for
  resolving the log directory/level/current file; it should be sequenced after
  that change lands, or built against its planned public API.
- No existing command output, flags, or exit codes change.

## Capabilities

### New Capabilities

- `cli-info`: An `info` subcommand that reports bonsai's own runtime/install
  paths and metadata (log directory, current log file, managed root path, log
  level, version).

### Modified Capabilities

(none — purely additive)

## Impact

- **Code**: new subcommand variant in `src/main.rs` (`Commands::Info`); a small
  dedicated module or function to assemble and print the info fields, reusing
  `worktree::managed_root()` and the logging subsystem's path/level resolution
  functions (exposed as `pub` from `src/logging.rs`).
- **Dependencies**: none new; reuses `worktree` and `logging` modules and
  existing `CARGO_PKG_VERSION` via `env!()`.
- **Sequencing**: depends on `add-structured-logging` landing first (or its
  `src/logging.rs` API being stable enough to call) for the log directory, log
  level, and current log file fields.
- **CLI surface**: adds one new subcommand; no existing subcommands or flags
  change.
