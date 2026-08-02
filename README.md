<div align="center">

# 🌳 Bonsai

<h3 align="center">Instant, clean git worktrees on demand.</h3>

Bonsai manages a pool of git worktrees so you can jump between tasks without
ever stashing, committing half-finished work, or waiting for a fresh clone. Run
`bs` and you get a clean slot at the current HEAD — ready to use, already on
disk. When you're done, put it back in the pool; the next call reuses it in
milliseconds.

</div>

## 🚀 Quick Start

```
$ bs get
🌳 ~/.bonsai/myrepo/a1b2c3d4
```

```
$ bs list
available  ~/.bonsai/myrepo/a1b2c3d4
in use     ~/.bonsai/myrepo/b5c6d7e8 (main)      ⚙2
in use     ~/.bonsai/myrepo/c9d0e1f2 (my-feature)
```

```
$ bs help
```

## tmux Integration

`bs get --tmux-session` creates (or reuses) a tmux session rooted at the
provisioned worktree slot, and attaches your terminal to it:

```bash
# Default session name: 🌳 <repo-name> (<branch-display>)
$ bs get --tmux-session

# Combine with -b/-B/<branch> to name the session after that branch
$ bs get -b my-feature --tmux-session

# Use a custom session name instead of the derived default
$ bs get --tmux-session=my-custom-session

# Create the session in the background without attaching/switching to it
$ bs get --tmux-session --no-attach
```

Notes:

- `--tmux-session` requires `tmux` to be installed and on `PATH`; `bs get` exits
  non-zero with an actionable error if it is not found. Plain `bs get` (no
  `--tmux-session`) never checks for or invokes `tmux`.
- `--no-attach` requires `--tmux-session`.
- `--no-attach` sessions are **not** automatically cleaned up — use
  `tmux kill-session -t <name>` when you're done with one.

> **Note:** the external `worktree-get` dotfiles script currently re-implements
> this same tmux session-naming/creation logic itself. A follow-up change will
> slim it down to a thin wrapper that passes `--tmux-session` through to
> `bs get` instead.

## Install

**Prerequisites:** [mise](https://mise.jdx.dev/).

```bash
git clone https://github.com/ddbas/bonsai.git
cd bonsai
mise run install
```

Make sure `~/.local/bin` is on your `PATH`.

## Logging

Bonsai automatically logs detailed information about its operations to help
debug issues. Logs are written only to a file — never to stdout or stderr — so
your normal command output stays clean.

### Log Location

Logs are written to the platform-appropriate log directory:

- **Linux**: `$XDG_STATE_HOME/bonsai/logs` (or `~/.local/state/bonsai/logs` if
  `XDG_STATE_HOME` is not set)
- **macOS**: `~/Library/Logs/bonsai` (fallback behavior)
- **Windows**: `%LOCALAPPDATA%/bonsai/logs`

Log files are automatically rotated daily with the prefix `bonsai.log`. For
example: `bonsai.log.2026-07-21`.

### Log Level

The default log level is `info`. To override it, use the global `--log-level`
flag before any subcommand:

```bash
bs --log-level debug get          # Log at debug level
bs --log-level warn list          # Log at warn level (less verbose)
bs --log-level trace help         # Log everything (most verbose)
```

Valid levels are: `trace`, `debug`, `info` (default), `warn`, `error`.

### Retention

Old log files are automatically pruned to keep the most recent 7 daily log
files. This bounds disk usage and prevents unbounded log growth.

## Getting Runtime Information

### `bs info` – View Bonsai Configuration & Paths

The `bs info` command prints bonsai's own runtime paths and metadata, useful for
debugging or scripting.

```bash
$ bs info
version: 0.1.0
log level: info
log directory: ~/Library/Application Support/bonsai/logs
current log file: ~/Library/Application Support/bonsai/logs/bonsai.log.2026-07-21
managed root: ~/.bonsai
```

**Output fields:**

- `version`: The bonsai version being run
- `log level`: The effective log level for this invocation (default is `info`,
  or overridden via `--log-level`)
- `log directory`: The resolved log directory path
- `current log file`: The path to today's active log file (may not yet exist if
  logging has never been initialized)
- `managed root`: The root directory where all bonsai-managed worktree pools are
  stored (`~/.bonsai`)

All paths are tilde-abbreviated (e.g., `~/` for the user's home directory) and
formatted as plain `key: value` lines, making the output easy to parse with
`grep` or shell scripts:

```bash
$ bs info | grep 'log directory'
log directory: ~/Library/Application Support/bonsai/logs
```

The `bs info` command performs no filesystem writes and will succeed even if
logging has never been initialized, making it safe to use as a first debugging
step when bonsai encounters issues.
