## Context

`add-structured-logging` introduces file-only logging with a resolved log
directory, an effective log level, and daily-rotating log files, none of which
are currently visible to users through any command. Separately, bonsai already
resolves a well-known managed root (`~/.bonsai`, via `worktree::managed_root()`)
that is never printed anywhere either. Users debugging an issue, or filing a bug
report, currently have no built-in way to discover these paths — they'd need to
read source or guess platform conventions. This change adds a small, read-only
`bs info` subcommand that surfaces this existing state.

## Goals / Non-Goals

**Goals:**

- Provide a single command that prints bonsai's own resolved runtime
  paths/metadata: log directory, current day's log file, managed root
  (`~/.bonsai`), effective log level, and bonsai version.
- Keep the output simple, stable, and easy to both read by a human and
  parse/grep by a script.
- Reuse existing path-resolution logic (`worktree::managed_root()`, the logging
  subsystem's directory/level resolution) rather than duplicating it.
- Work correctly whether or not the current directory is inside a bonsai-managed
  repo (this is global, install-level info, not per-repo).

**Non-Goals:**

- No health checks / validation (e.g. "is the log directory writable?", "is
  `~/.bonsai` on this disk?") — that's a future `doctor`-style command, not this
  one.
- No JSON or other structured output format for v1 — plain text is sufficient
  given the small, fixed field set.
- No per-repo information (worktree pool contents, slot status) — that's already
  covered by `bs list`/`bs current`.
- No listing of individual log files or their sizes/ages — just the directory
  and the current file.

## Decisions

### 1. Command shape: a new top-level subcommand, `bs info`

Added as a new `Commands::Info` variant alongside `Get`, `List`, `Current`,
`Lock`, `Unlock`. It takes no arguments and no flags beyond the existing global
`--log-level` (which still affects the _effective log level_ field reported,
letting users confirm an override took effect). Alternative considered: folding
this into `bs current` or `bs list`. Rejected — those commands are scoped to
worktree state for the current repo/CWD, whereas `info` is global, install-level
metadata; conflating the two would make both commands' output harder to parse
and reason about.

### 2. Output format: plain `key: value` lines to stdout

One field per line, stable key names, e.g.:

```
version: 0.1.0
log level: info
log directory: /Users/you/.local/state/bonsai/logs
current log file: /Users/you/.local/state/bonsai/logs/bonsai.log.2026-07-21
managed root: /Users/you/.bonsai
```

This mirrors the simplicity of existing bonsai output (e.g. `bs current`
printing a tilde-abbreviated path) and is trivially greppable
(`bs info | grep 'log directory'`) without requiring a JSON parser for a 5-line
output. Alternative considered: JSON output (`--json` flag or default). Rejected
for v1 — adds a serialization dependency/format-versioning concern for a tiny,
fixed set of fields; can be added later as an additive `--json` flag without
breaking the plain-text default.

### 3. Path resolution: expose small `pub` accessors from `src/logging.rs`, reuse `worktree::managed_root()`

`add-structured-logging`'s `src/logging.rs` should expose:

- a function returning the resolved log directory `PathBuf` (independent of
  whether `init()` succeeded — same resolution logic, no I/O side effects
  required to just report the path)
- a function returning the current day's log file path given the directory (the
  same naming scheme `init()`/rotation already uses) `bs info` calls these plus
  `worktree::managed_root()` and prints tilde-abbreviated paths via the existing
  `worktree::tilde_path()` helper for consistency with `bs current`/`bs list`.
  Alternative considered: having `bs info` re-implement path resolution
  independently. Rejected — would risk drift between what `info` reports and
  what logging actually does; single source of truth in `src/logging.rs` is
  safer.

### 4. Effective log level display: report the resolved value, not raw CLI input

`info` prints the level that was actually resolved for this invocation (default
`info`, or the `--log-level` override value, already validated by clap as a
`LogLevel` enum) — i.e. `bs --log-level debug info` prints `log level: debug`.
Alternative considered: showing only the default and ignoring the flag. Rejected
— the whole point of surfacing this field is to let users confirm what level is
active, including confirming an override took effect.

### 5. No file/directory existence requirement for reporting

`bs info` reports resolved paths whether or not the log directory has been
created yet (e.g. on a machine that has never had logging successfully
initialized due to a permissions issue) — it reports what bonsai _would_ use,
not what currently exists on disk. The "current log file" field is computed the
same way (today's expected filename), not verified against the filesystem.
Alternative considered: erroring or omitting fields when a path doesn't exist
yet. Rejected — `info` is meant to work reliably as a first debugging step even
when logging itself failed to initialize; a lookup that could fail would
undermine that.

## Risks / Trade-offs

- [Risk] `bs info`'s log-related fields depend on `add-structured-logging`
  landing first and its `src/logging.rs` API remaining stable → Mitigation:
  sequence this change after `add-structured-logging` is implemented; if
  `add-structured-logging`'s internal function names change, only `bs info`'s
  call sites need updating, not its output format.
- [Risk] Plain `key: value` text output could later need machine-readable
  structure (e.g. for tooling) → Mitigation: keys and formatting are chosen to
  already be easily grep/awk-able; a `--json` flag can be added later without
  breaking the default text output (decision 2).
- [Risk] Reporting a "current log file" path that doesn't yet exist on disk
  could be confusing if a user expects `info` to only show real files →
  Mitigation: this is an explicit, documented design choice (decision 5);
  phrasing in `--help`/README should clarify these are _resolved paths bonsai
  uses_, not a guarantee the file currently exists.
