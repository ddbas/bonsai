## 1. CLI surface

- [x] 1.1 Add `tmux_session: Option<Option<String>>`-style field (or
      `num_args(0..=1)` + `default_missing_value`) to `Commands::Get` for
      `--tmux-session [<NAME>]`.
- [x] 1.2 Add `no_attach: bool` field to `Commands::Get` for `--no-attach`,
      declared with `requires = "tmux_session"`.
- [x] 1.3 Write doc comments for both new fields matching the style of existing
      `Get` fields (used for `--help` output).
- [x] 1.4 Manually verify `bs get --help` renders both flags correctly and that
      `bs get --no-attach` (without `--tmux-session`) fails with clap's usage
      error.

## 2. Session name derivation

- [x] 2.1 Add a helper (e.g. `worktree::tmux_session_name` or a new
      `src/tmux.rs`) that computes `🌳 <repo-name> (<branch-display>)` from
      `repo_slug()` and the resolved branch (reusing the same branch-name string
      already computed for the existing `(branch)` stdout suffix), falling back
      to the literal `detached` when no branch was requested.
- [x] 2.2 Wire the optional `--tmux-session` value: empty/missing → derived
      name; non-empty → used verbatim.
- [x] 2.3 Unit test the derivation helper: existing branch, `-b`/`-B` branch,
      and no-branch (`detached`) cases, without requiring a real tmux binary.

## 3. tmux invocation

- [x] 3.1 Add a helper module/function wrapping `tmux has-session`,
      `tmux new-session -ds <name> -c <path>`,
      `tmux switch-client -t     <name>`, and `tmux attach-session -t <name>`
      via `std::process::Command`, consistent with existing `git_cmd()`-style
      invocation patterns in `src/worktree/mod.rs`.
- [x] 3.2 Add an up-front check for `tmux` on `PATH` (only performed when
      `--tmux-session` is passed) that exits non-zero with an actionable error
      if missing.
- [x] 3.3 Implement create-or-reuse: call `has-session` first, only run
      `new-session` if it reports no existing session.
- [x] 3.4 Implement attach behavior: `switch-client` when `$TMUX` env var is
      set, `attach-session` otherwise; skip entirely when `--no-attach` is
      passed.

## 4. Wire into `bs get`

- [x] 4.1 In `main.rs`'s `Commands::Get` match arm, after resolving `path` and
      `branch_name` (and printing the existing `🌳 <path> (<branch>)` line),
      invoke the tmux helper when `tmux_session` is `Some(..)`.
- [x] 4.2 Confirm the default no-flag path (`tmux_session: None`) takes zero new
      code paths / makes zero tmux calls (regression check against current
      `bs get` behavior).
- [x] 4.3 Resolve the open question from design.md: decide whether to print the
      resolved session name in stdout, and implement accordingly.

## 5. Tests

- [x] 5.1 Add integration tests alongside `tests/worktree_get.rs` for:
      `bs get --tmux-session` creates a session with the default name; reruns
      reuse it; `--tmux-session=<name>` uses the custom name; `--no-attach`
      creates without attaching; `--no-attach` without `--tmux-session` fails at
      parse time.
- [x] 5.2 Skip (not fail) tmux-dependent integration tests when `tmux` is not
      found on `PATH`, matching existing environment-dependent test patterns in
      this codebase.
- [x] 5.3 Run the full test suite and confirm no regressions in existing
      `bs get` tests (`tests/worktree_get.rs`, `get-branch-options`,
      `get-checkout-existing-branch` coverage).

## 6. Docs

- [x] 6.1 Update README usage examples to document `--tmux-session` and
      `--no-attach`.
- [x] 6.2 Note in `--help`/README that `--no-attach` sessions are not
      automatically cleaned up.
- [x] 6.3 (Follow-up, optional in this change) Note in a TODO/README that
      `worktree-get` can be slimmed down to pass `--tmux-session` through to
      `bs get` instead of doing its own tmux calls.
