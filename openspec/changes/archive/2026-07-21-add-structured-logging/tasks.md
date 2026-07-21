## 1. Dependencies

- [x] 1.1 Add `tracing`, `tracing-subscriber` (with `env-filter` feature), and
      `tracing-appender` to `[dependencies]` in `Cargo.toml`
- [x] 1.2 Run `cargo build` to confirm the new dependencies compile cleanly

## 2. Logging Module

- [x] 2.1 Create `src/logging.rs` with a `LogLevel` enum
      (trace/debug/info/warn/error) implementing `clap::ValueEnum` and a
      conversion to `tracing::Level`/`tracing_subscriber::filter::LevelFilter`
- [x] 2.2 Implement a function to resolve the platform log directory
      (`dirs::state_dir().or_else(dirs::data_local_dir)` joined with
      `bonsai/logs`), creating the directory (and parents) if missing
- [x] 2.3 Implement
      `init(level: LogLevel) -> Option<tracing_appender::non_blocking::WorkerGuard>`
      that:
  - builds a `tracing_appender::rolling::RollingFileAppender` with
    `Rotation::DAILY` and prefix `bonsai.log` in the resolved log directory
  - wraps it with `tracing_appender::non_blocking`
  - installs a `tracing_subscriber` `fmt` layer writing only to the non-blocking
    file writer, filtered by the resolved level
  - returns `None` (and does not panic) if directory creation or file open
    fails, ensuring the CLI proceeds without logging
- [x] 2.4 Implement a retention-pruning function that lists files in the log
      directory matching the `bonsai.log.*` prefix, sorts by date, and deletes
      files beyond a fixed retention count (e.g. keep 7); call it once during
      `init()` after successful setup
- [x] 2.5 Wire `src/logging.rs` into `src/lib.rs` (module declaration)

## 3. CLI Integration

- [x] 3.1 Add a global `--log-level <LEVEL>` flag to the top-level `Cli` struct
      in `src/main.rs`, typed as `logging::LogLevel`, defaulting to `info`
- [x] 3.2 Call `logging::init(cli.log_level)` at the top of `main()`, before
      subcommand dispatch, and hold the returned `WorkerGuard` in a local
      binding for the lifetime of `main()`
- [x] 3.3 Audit `main.rs` (and any subcommand handlers) for early-exit paths
      (e.g. `std::process::exit`) and ensure the guard is dropped or logs are
      flushed before those exits, per the design's guard-lifetime constraint

## 4. Instrumentation

- [x] 4.1 Add `tracing::info!`/`debug!` events around key `worktree` module
      operations: slot provisioning/reuse, branch checkout/reset, lock/unlock
- [x] 4.2 Add `tracing::warn!`/`error!` events at points where operations
      currently return `Err`/fail (e.g. git subprocess failures, missing
      branches) without changing the returned error values or user-facing output
- [x] 4.3 Add a `tracing::debug!` event logging the parsed CLI invocation
      (command + args, excluding nothing sensitive) at the start of `main()`

## 5. Testing

- [x] 5.1 Add unit tests for the log directory resolution function (e.g.
      respects `XDG_STATE_HOME` when set, falls back correctly when unset)
- [x] 5.2 Add unit tests for the retention-pruning function (keeps most recent
      N, deletes older ones) using a temp directory (`tempfile`, already a
      dev-dependency)
- [x] 5.3 Add a test verifying `logging::init` returns `None` (not a panic) when
      given a non-writable directory
- [x] 5.4 Add/extend an integration or CLI test confirming that running a
      command with default settings produces no log output on stdout/stderr
- [x] 5.5 Add a CLI test confirming `--log-level` accepts valid values and
      rejects invalid ones with a standard clap error

## 6. Documentation

- [x] 6.1 Update `README.md` (or add a short "Logging" section) documenting the
      default log location per platform, default level, the `--log-level` flag,
      and rotation/retention behavior
- [x] 6.2 Update the `bs help`/clap doc comment for the new `--log-level` flag
      to match the behavior described in the spec
