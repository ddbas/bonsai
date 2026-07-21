## 1. Prerequisites

- [x] 1.1 Confirm `add-structured-logging` has landed (or its `src/logging.rs`
      API is stable enough to depend on) before starting implementation

## 2. Logging API Surface

- [x] 2.1 Expose a `pub fn log_dir() -> PathBuf` (or equivalent) in
      `src/logging.rs` that returns the resolved log directory without
      performing any I/O or side effects
- [x] 2.2 Expose a `pub fn current_log_file(dir: &Path) -> PathBuf` (or
      equivalent) in `src/logging.rs` that computes today's expected log file
      path using the same naming scheme as the rolling appender
- [x] 2.3 Expose a way to read the effective/resolved `LogLevel` for the current
      invocation (e.g. a getter set during `init()`, or simply re-deriving it
      from the parsed CLI args passed into `info`)

## 3. CLI Integration

- [x] 3.1 Add an `Info` variant to the `Commands` enum in `src/main.rs`, taking
      no arguments
- [x] 3.2 Implement the `Info` command handler: gather version
      (`env!("CARGO_PKG_VERSION")`), effective log level, log directory, current
      log file path, and `worktree::managed_root()`
- [x] 3.3 Format and print each field as `key: value` on its own line, using
      `worktree::tilde_path()` for path values
- [x] 3.4 Ensure the `Info` handler performs no filesystem writes (no
      directory/file creation) as a side effect

## 4. Testing

- [x] 4.1 Add a CLI test asserting `bs info` exits 0 and prints all five
      expected field labels (`version`, `log level`, `log directory`,
      `current log file`, `managed root`)
- [x] 4.2 Add a CLI test asserting `bs --log-level debug info` reports
      `log level: debug`
- [x] 4.3 Add a test asserting `bs info` reports paths without creating the log
      directory when it doesn't already exist (e.g. run in an isolated
      `$HOME`/`BONSAI_ROOT` temp dir and assert the directory is absent after
      the command runs)
- [x] 4.4 Add a unit test for `logging::current_log_file()` confirming it
      matches the filename pattern produced by the rolling appender

## 5. Documentation

- [x] 5.1 Update `README.md` and/or `--help` doc comments to describe `bs info`
      and its output fields
