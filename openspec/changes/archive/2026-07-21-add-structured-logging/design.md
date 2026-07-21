## Context

Bonsai (`bs`) is a Rust CLI (`clap`-based) that manages git worktree pools. It
currently has zero logging: all diagnostics are ad hoc `eprintln!`/`anyhow`
error messages surfaced directly to the user. There is no durable trail of what
commands did, which makes debugging pool-state issues (stale locks, unexpected
slot resets, git subprocess failures) difficult after the fact. `dirs` is
already a dependency and is the natural source for platform-appropriate
directories. The CLI's primary output (paths, tables, statuses) is consumed by
scripts and terminals, so stdout/stderr must remain exactly as they are today —
logging must be strictly additive and file-only.

## Goals / Non-Goals

**Goals:**

- Provide a single, idiomatic logging setup usable from every module via
  `tracing::{info,debug,warn,error,trace}!` macros.
- Default to `info` level with zero configuration required.
- Write logs only to a file in the platform-appropriate log/state directory;
  never to the terminal.
- Rotate log files automatically and bound retention so logs don't grow
  unbounded.
- Let users override the level via a global CLI flag, without needing an env
  var.
- Fail gracefully: if the log directory/file can't be created or written, the
  CLI still runs and produces its normal output.

**Non-Goals:**

- Structured/JSON log output (plain human-readable lines are sufficient for
  now).
- Remote log shipping, telemetry, or crash reporting.
- Per-module or per-target level configuration beyond a single global level.
- Log viewing/tailing UX inside bonsai itself (users can `tail`/`less` the file
  directly).
- Changing any existing stdout/stderr output or exit codes.

## Decisions

### 1. Logging stack: `tracing` + `tracing-subscriber` + `tracing-appender`

This is the de facto standard for modern Rust CLIs and libraries (used by
`cargo`, `rustup`, `tokio` ecosystem tools). `tracing` gives structured, leveled
events with near-zero cost when disabled, and composes cleanly with
`tracing-subscriber`'s layered filtering. `tracing-appender` provides
`rolling::RollingFileAppender` for time-based rotation and a `non_blocking`
writer so file I/O doesn't add latency to command execution. Alternative
considered: `log` + `env_logger` + `flexi_logger`/`log4rs`. Rejected because
`log` is a thinner facade with weaker ecosystem support for rotation, and
`tracing` is now the more actively maintained, widely adopted choice for new
projects (better filtering, span support if bonsai grows async/concurrent
operations later, e.g. via existing `tokio` dev-dependency).

### 2. Log location: XDG-style directory via `dirs` crate

Use `dirs::state_dir()` when available (Linux: `$XDG_STATE_HOME` or
`~/.local/state`), falling back to `dirs::data_local_dir()` on platforms without
a dedicated state dir (macOS has no XDG state dir concept; use
`~/Library/Logs/bonsai` via a manual join since `dirs` doesn't expose a "Logs"
special dir on macOS — construct `dirs::home_dir()?.join("Library/Logs/bonsai")`
on macOS specifically, or simply reuse `data_local_dir()/bonsai/logs` uniformly
across platforms for simplicity and to avoid macOS-specific branching).
Decision: use a single cross-platform helper —
`dirs::state_dir().or_else(dirs::data_local_dir)` joined with `bonsai/logs` —
rather than hand-rolling per-OS paths, since correctness/consistency matters
more than following macOS's `Library/Logs` convention exactly, and it keeps the
implementation in one small function with no `#[cfg(...)]` branching.
Alternative considered: the `directories`/`directories-next` crate, which has a
`ProjectDirs` API purpose-built for XDG log/data/config split (including a
dedicated macOS `Logs` path). Rejected only to avoid adding a second,
overlapping crate to `dirs` (already a dependency); may reconsider if `dirs`'s
coverage proves insufficient.

### 3. Default level and override mechanism

Default level is `info`. Override precedence: explicit `--log-level <level>` CLI
flag (highest) > built-in default `info` (no env var layer, to keep behavior
simple and documented in one place — the flag). The flag is defined once on the
top-level `Cli` struct in `src/main.rs` (global, so it's available
before/alongside subcommand parsing) and threaded into the logging init call
before any subcommand logic runs. Alternative considered: also honoring
`RUST_LOG`/`BONSAI_LOG` env var via `EnvFilter::try_from_default_env()`.
Rejected for v1 to keep the override surface single-sourced (the flag) and
predictable; can be added later as an additional fallback layer without breaking
the flag's precedence.

### 4. Rotation policy

Use `tracing_appender::rolling::RollingFileAppender` with `Rotation::DAILY` and
a fixed filename prefix (`bonsai.log`), producing dated files like
`bonsai.log.2026-07-21`. Retention: prune files older than a fixed count (e.g.
keep the last 7 daily files) on startup, implemented as a small explicit cleanup
pass in `src/logging.rs` (tracing-appender itself does not prune old files).
Alternative considered: size-based rotation. Rejected because `tracing-appender`
only supports time-based rotation natively; size-based would require a
third-party crate (`file-rotate`) and adds complexity not justified by bonsai's
expected log volume (a lightweight CLI invoked frequently but briefly).

### 5. Non-blocking writer and shutdown

Use `tracing_appender::non_blocking(rolling_appender)` to avoid blocking command
execution on log I/O, keeping the returned `WorkerGuard` alive for the duration
of `main()` (held as a local binding, dropped at end of `main`) so buffered log
lines are flushed before process exit. Alternative considered: blocking writes
directly. Rejected — bonsai commands are short-lived and I/O-sensitive (git
subprocess calls), so avoiding any avoidable blocking on log writes is
preferred, and the guard-lifetime pattern is the documented idiomatic approach.

### 6. Failure handling

If the log directory cannot be created or the file cannot be opened,
`src/logging.rs::init()` returns a `Result` that `main()` matches on: on `Err`,
print nothing to the user (or at most a single best-effort `eprintln!` in debug
builds) and continue running with a no-op subscriber (or no subscriber installed
at all), rather than aborting. This guarantees logging is strictly best-effort
and never blocks the CLI's primary function.

## Risks / Trade-offs

- [Risk] Log directory permissions or a read-only filesystem prevent log
  creation → Mitigation: `init()` treats this as non-fatal; CLI proceeds without
  file logging (decision 6).
- [Risk] Retention cleanup (decision 4) runs on every invocation and could add
  filesystem-scan overhead → Mitigation: keep the cleanup cheap (single
  `read_dir` over the log directory, filename-prefix match) and only scan, never
  on the hot path of command logic (run it after logging init, not per log
  line).
- [Risk] `--log-level` accepts invalid input → Mitigation: validate via `clap`'s
  `ValueEnum`/`value_parser` against the known level set
  (trace/debug/info/warn/error) so invalid values fail fast with a standard clap
  error, not a silent fallback.
- [Risk] Choosing `dirs` over `directories` for log paths (decision 2) means no
  dedicated macOS "Logs" convention → Mitigation: documented as an explicit,
  revisitable trade-off; behavior is still correct and discoverable (single
  consistent path across platforms), just not macOS-idiomatic.
- [Risk] Non-blocking writer drops final log lines if the `WorkerGuard` is
  dropped early (e.g. early `std::process::exit`) → Mitigation: audit `main.rs`
  for any `process::exit` calls and ensure the guard is dropped (or logs
  flushed) before any early exit path; document this constraint in
  `src/logging.rs`.
