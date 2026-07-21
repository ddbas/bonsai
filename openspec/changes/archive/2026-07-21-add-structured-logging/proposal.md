## Why

Bonsai currently has no logging facility: diagnosing failures (git errors,
filesystem issues, pool-slot corruption) requires reproducing the problem
interactively, and there is no durable record of what the CLI did across
invocations. As bonsai grows more commands and touches more state (worktree
pool, locks, config), we need a persistent, low-noise log trail that doesn't
clutter the terminal but is available for debugging when something goes wrong.

## What Changes

- Add a logging subsystem based on `tracing` + `tracing-subscriber` +
  `tracing-appender`, instrumented throughout `src/` (main, worktree,
  config/pool operations).
- Default log level is `info`; logs are written only to a file, never to
  stdout/stderr, so normal CLI output (paths, status tables) stays clean.
- Log files are written to the OS-appropriate XDG/platform log directory (e.g.
  `$XDG_STATE_HOME/bonsai/` or `~/.local/state/bonsai/` on Linux,
  `~/Library/Logs/bonsai/` on macOS), resolved via the `dirs` crate (already a
  dependency) with a bonsai-specific fallback.
- Log files rotate automatically (daily rotation via
  `tracing-appender::rolling`) to bound disk usage, with old files retained up
  to a small, documented count/age.
- Add a global CLI flag (e.g. `--log-level <level>`) available on `bs` and all
  subcommands to override the default `info` level
  (trace/debug/info/warn/error), taking precedence over any default/env
  configuration.
- Logging initialization failures (e.g. unable to create log directory) must not
  crash the CLI; the tool falls back to running without file logging and
  continues normal operation.

## Capabilities

### New Capabilities

- `cli-logging`: Structured logging subsystem for the bonsai CLI — file-only
  output routed to the platform log directory, default `info` level, global
  `--log-level` override flag, and daily-rotating log files with retention.

### Modified Capabilities

(none — this introduces new behavior without changing existing command
contracts)

## Impact

- **Dependencies**: add `tracing`, `tracing-subscriber` (with `env-filter`
  feature), `tracing-appender` to `Cargo.toml`.
- **Code**: new `src/logging.rs` module for setup/init; `src/main.rs` gains a
  global `--log-level` arg on the `Cli` struct and calls the logging init before
  dispatching subcommands; `src/worktree/mod.rs` and other modules gain
  `tracing::{info,debug,warn,error}` instrumentation at key operations (slot
  provisioning, lock/unlock, git invocations).
- **Filesystem**: introduces a new log directory under the user's XDG state/log
  path; no existing files or directories are affected.
- **CLI surface**: adds one new global flag; no existing flags or output formats
  change.
