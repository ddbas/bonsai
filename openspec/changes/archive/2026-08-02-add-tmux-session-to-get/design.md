## Context

`bs get` currently provisions/reuses a bonsai pool slot and prints its path
(plus the branch name when `-b`/`-B`/positional `<branch>` was used). Users who
want a tmux session for that slot today rely on the external `worktree-get`
dotfiles script, which shells out to `bs get`, parses its stdout to recover the
slot path and branch, and then does:

```sh
session_name="🌳 ${repo_name} (${branch_display})"
if ! tmux has-session -t="$session_name" 2>/dev/null; then
  tmux new-session -ds "$session_name" -c "$worktree_path"
fi
tmux switch-client -t "$session_name"
```

This change moves that behavior (minus the fzf branch picker, which stays
external) into `bs get` itself, behind an opt-in flag, so any caller gets it
without stdout-scraping.

## Goals / Non-Goals

**Goals:**

- Add opt-in tmux session creation to `bs get` via `--tmux-session`.
- Match `worktree-get`'s existing session-naming convention by default:
  `🌳 <repo-name> (<branch-display>)`.
- Let callers override the session name.
- Let callers choose whether the invoking terminal is attached/switched to the
  session, without requiring a separate always-present flag.
- Leave default `bs get` (no flags) behavior completely unchanged.

**Non-Goals:**

- Replacing or reimplementing the `worktree-get` fzf branch-picker UX itself —
  it will simply become a thinner wrapper that passes `--tmux-session` through
  to `bs get` instead of doing tmux calls itself (follow-up, not part of this
  change).
- Managing tmux session lifecycle beyond creation (e.g. killing sessions on
  `bs unlock` or slot recycling) — out of scope.
- Supporting tmux session creation for `bs list`/`bs current`/other subcommands
  — this change only touches `bs get`.

## Decisions

### 1. Gate the attach/no-attach choice with clap's `requires`, not a manual check

`--no-attach` is declared with `requires = "tmux_session"` in the clap
`Args`/`Subcommand` derive. This makes "only valid alongside `--tmux-session`" a
declarative, self-documenting constraint enforced by clap itself (consistent
with how `-b`/`-B`/positional `branch` already use `conflicts_with`), rather
than a hand-rolled runtime check with an ad hoc error message.
`bs get --no-attach` without `--tmux-session` fails at argument-parsing time
with clap's standard "the following required arguments were not provided"
message.

**Alternative considered**: A single tri-state flag like
`--tmux-session=name:attach` or `--tmux-session=name,no-attach` packed into one
value. Rejected — harder to discover via `--help`, awkward to parse, and
inconsistent with how the rest of the CLI expresses independent booleans as
separate flags.

**Alternative considered**: `--attach`/`--no-attach` pair (both requiring
`tmux_session`, mutually exclusive) with attach as the default. Rejected in
favor of a single `--no-attach` flag — with only two states and a clear default
(attach), one flag with a `false`-leaning name is simpler than two mutually
exclusive flags for the same boolean.

### 2. `--tmux-session` takes an optional value (bare flag or `--tmux-session=NAME`)

Declared with `num_args(0..=1)` and a sentinel `default_missing_value` (e.g.
empty string) so both `bs get --tmux-session` (default name) and
`bs get --tmux-session=my-name` (custom name) parse without needing a second
flag like `--tmux-session-name`. Post-parse, an empty/sentinel value means
"derive the default name"; any other value is used verbatim.

**Alternative considered**: Separate `--tmux-session-name <NAME>` flag, required
to be paired with a boolean `--tmux-session`. Rejected — two flags for one
concern is exactly the pattern the user wanted to avoid, and clap's
optional-value support removes the need for it.

### 3. Default session name is derived, not passed in by the caller

When no explicit name is given, `bs get` computes
`🌳 <repo-name> (<branch-display>)` itself:

- `repo-name`: same value already computed by `worktree::repo_slug()`
  (second-to-last path component convention used elsewhere in this codebase and
  mirrored by `worktree-get`'s `basename "$(dirname "$worktree_path")"`).
- `branch-display`: the resolved branch name when `<branch>`, `-b`, or `-B` was
  used (same string already used for the `(branch)` suffix in `bs get`'s
  existing stdout output); the literal `detached` when none was requested, so
  the naming convention stays unambiguous and collision-resistant even for plain
  `bs get --tmux-session` with no branch flags.

This keeps the convention in one place (Rust) instead of re-deriving it from
parsed stdout in shell scripts, and keeps `worktree-get` (or any other caller)
able to simply omit a name and get the same convention as before.

### 4. Session creation/attachment mirrors `worktree-get`'s tmux calls exactly

- `tmux has-session -t="$name"` to check for an existing session (idempotent
  reuse, matching current shell script behavior — running
  `bs get --tmux-session` twice for the same branch does not create duplicate
  sessions or error).
- `tmux new-session -ds "$name" -c "$worktree_path"` to create it detached if it
  doesn't exist yet.
- Attach behavior (unless `--no-attach`):
  - If already inside a tmux client (`$TMUX` env var set):
    `tmux switch-client -t "$name"`.
  - Otherwise: `tmux attach-session -t "$name"` (switch-client only works from
    inside an existing tmux client; a plain shell invocation of
    `bs get --tmux-session` needs `attach-session` instead).
- All tmux invocations go through `std::process::Command`, consistent with
  existing `git` invocations in `src/worktree/mod.rs` (no new crate dependency).

**Alternative considered**: Always use `switch-client`. Rejected — it fails
outside of an existing tmux client, which is the common case for a first
`bs get --tmux-session` invocation from a plain shell.

### 5. Missing `tmux` binary is a hard error, only when the flag is used

`bs get --tmux-session` checks for `tmux` on `PATH` up front (mirroring the
existing `command -v tmux` check in `worktree-get`) and exits non-zero with an
actionable message if absent. Plain `bs get` never checks for `tmux` at all,
preserving today's zero-dependency default path.

## Risks / Trade-offs

- **[Risk]** Duplicating tmux-invocation logic that already exists in the
  external `worktree-get` script could drift out of sync if one is updated
  without the other. → **Mitigation**: this change intentionally makes `bs get`
  the single source of truth for the naming convention and tmux calls; a
  documented follow-up is to slim `worktree-get` down to just the fzf picker +
  `bs get --tmux-session` passthrough, removing its own tmux calls entirely.
- **[Risk]** CI/test environments may not have `tmux` installed. →
  **Mitigation**: tmux-session integration tests skip (not fail) when `tmux` is
  not found on `PATH`, matching the pattern already used for other
  environment-dependent checks in this codebase; core flag-parsing/name-
  derivation logic is covered by unit tests that don't require a real tmux
  binary.
- **[Risk]** `--no-attach` sessions created in the background could leak
  (accumulate) if users forget about them. → **Mitigation**: out of scope for
  this change (no new lifecycle management), but worth noting in `--help` text
  that `--no-attach` sessions are not automatically cleaned up;
  `tmux kill-session` remains the user's responsibility.

## Open Questions

- Should the derived session name be surfaced in `bs get`'s stdout (e.g.
  appended after the existing `🌳 <path> (<branch>)` line) so scripts/users can
  see which session was created/reused without re-deriving it? Leaning yes for
  parity with existing output conventions; to be finalized during
  implementation.
