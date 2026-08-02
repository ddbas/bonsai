## Why

Developers already juggle multiple bonsai worktrees for parallel branches, and
the common next step after `bs get` is to open a tmux session in the newly
provisioned slot — today done by hand or via the separate `worktree-get`
dotfiles script. Folding that step into `bs get` itself lets any caller (shell
alias, script, or the `worktree-get` picker) opt into a ready-to-use tmux
session in one command, using the same session-naming convention already
established by `worktree-get`.

## What Changes

- Add a `--tmux-session [<NAME>]` option to `bs get`. When passed, a tmux
  session is created (or reused if one with that name already exists) rooted at
  the just-provisioned worktree path.
  - With no value, the session name defaults to the `worktree-get` naming
    convention: `🌳 <repo-name> (<branch-display>)`, where `<branch-display>` is
    the resolved branch name if one was checked out (via `<branch>`, `-b`, or
    `-B`), or a detached-HEAD label when no branch was requested.
  - With a value (`--tmux-session my-session`), that exact name is used instead.
- Add a `--no-attach` flag that is only valid alongside `--tmux-session`
  (enforced via clap's `requires`) and suppresses switching/attaching the
  current terminal to the created session — the session is created in the
  background and the command exits normally. Without `--no-attach`,
  `bs get --tmux-session` attaches to (or switches to, when already inside tmux)
  the session, mirroring `worktree-get`'s current behavior.
- Default behaviour of `bs get` (no flags passed) is unchanged: no tmux session
  is created or touched.
- `bs get --tmux-session` requires `tmux` to be installed and reachable on
  `PATH`; when it is not, the command SHALL exit non-zero with an actionable
  error instead of silently skipping tmux integration.

## Capabilities

### New Capabilities

- `get-tmux-session`: Optional tmux session creation/attachment for `bs get`,
  including default session naming, custom naming, and attach/no-attach control.

### Modified Capabilities

(none — no existing requirements change; this only adds new, opt-in behavior to
`bs get`)

## Impact

- Affected code: `src/main.rs` (new `Commands::Get` fields and CLI wiring),
  `src/worktree/mod.rs` (or a new `src/tmux.rs` module) for session-name
  derivation and tmux invocation.
- New runtime dependency: shelling out to the `tmux` binary (no new Rust crate
  dependency expected; existing `std::process::Command` usage pattern applies).
- Tests: new integration coverage alongside `tests/worktree_get.rs` exercising
  `--tmux-session` (default name, custom name) and `--no-attach`, using a way to
  fake/skip real tmux interaction in CI (e.g. a `TMPDIR`-scoped tmux socket or
  skipping when `tmux` is unavailable).
- Docs: update `bs get --help` text and README usage examples.
