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
▶ ~/.bonsai/myrepo/a1b2c3d4 (current)  available
  ~/.bonsai/myrepo/b5c6d7e8 (main)  in use
  ~/.bonsai/myrepo/c9d0e1f2 (my-feature)  locked
```

```
$ bs status ~/.bonsai/myrepo/b5c6d7e8
🌳 ~/.bonsai/myrepo/b5c6d7e8  (main)
status: in use

open processes:
  1234  node

uncommitted changes (1):
  M  src/main.rs
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

## Configuration

Bonsai has no configuration file of its own; repo-scoped behavior is configured
entirely via `git config`, under the `bonsai.*` namespace.

### `bonsai.copy` — carry git-ignored files into new/reused slots

A freshly provisioned or reused pool slot only ever contains git-tracked
content. Files you keep around but never commit — `.env`, local override
configs, IDE settings — don't automatically follow you into a new slot. The
multi-valued `bonsai.copy` git config key lets you declare, once, a list of
relative file paths that `bs get` should copy from the "origin" worktree (the
worktree you ran `bs get` from) into the provisioned slot, as the final step of
provisioning (after the slot has been created or reset and the branch checked
out).

```bash
# Add one entry (repeat --add for each additional file)
$ git config --add bonsai.copy .env
$ git config --add bonsai.copy config/local.json

# Inspect what's currently configured
$ git config --get-all bonsai.copy
.env
config/local.json
```

Set it locally (per-repo, `.git/config`) or globally (`~/.gitconfig`) — both are
honored, using git's normal config precedence/merging rules for multi-valued
keys.

Notes:

- Entries are relative file paths (not directories or globs); nested paths (e.g.
  `config/local.json`) have their parent directories created automatically in
  the slot if needed.
- **A listed file that doesn't exist in the origin worktree is silently
  skipped** — no error, no warning. `bonsai.copy` is a best-effort list, not a
  strict manifest, so don't mistake a missing copy for a bug; verify your
  configured entries with `git config --get-all bonsai.copy` and confirm the
  file actually exists in the origin worktree.
- When `bonsai.copy` is unset, `bs get` behaves exactly as before this feature
  existed — no extra filesystem operations are performed.

## Install

**Prerequisites:** [mise](https://mise.jdx.dev/).

```bash
git clone https://github.com/ddbas/bonsai.git
cd bonsai
mise run install
```

Make sure `~/.local/bin` is on your `PATH`.

### Agent Skill

To install the Bonsai CLI skill for agents, run:

```bash
npx skills add ddbas/bonsai --skill bonsai
```

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

## Reclaiming Disk Space

### `bs prune` – Delete Unused Pool Slots

Over time, unused pool slots accumulate on disk. `bs prune` reclaims that space
for the current repository:

1. Every managed pool slot is classified `available` / `in use` / `locked`,
   using the same rules as `bs list`/`bs status`.
2. The on-disk directory of every `available` slot is deleted. `in use` and
   `locked` slots are never touched.
3. `git worktree prune` is run once, so git deregisters the worktrees whose
   directories were just deleted. `bs prune` never edits git's worktree
   bookkeeping directly — that's entirely `git worktree prune`'s job.

```bash
$ bs prune
🗑️  pruned ~/.bonsai/myrepo/a1b2c3d4
🗑️  pruned ~/.bonsai/myrepo/e3f4a5b6  (my-feature)
```

If there is nothing to prune (no pool yet, or no `available` slots), `bs prune`
prints a friendly message and exits `0` without deleting anything. If deleting
one slot's directory fails, `bs prune` reports that failure, still prunes the
remaining available slots and runs `git worktree prune`, and exits non-zero.
