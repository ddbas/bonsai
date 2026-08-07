## Why

Managed worktrees provisioned by `bs get` always start from git's tracked state
only. Developers routinely keep git-ignored files (e.g. `.env`, local override
configs, IDE settings) in their main/origin worktree that a freshly provisioned
or reused pool slot doesn't have, forcing them to manually recreate these files
every time. There is no way to tell `bs` which ignored files should follow a
worktree into a new or reused slot.

## What Changes

- Add support for a `bonsai.copy` git config key (multi-valued) that lists file
  names/relative paths to copy from the "origin" worktree (the worktree `bs get`
  was invoked from) into the provisioned pool slot.
- Copy runs as the final step of `bs get`, after the slot has been created or
  reset (and after branch checkout), for both code paths: a brand-new slot
  (`git worktree add`) and a reused existing slot (`git worktree lock`/reset
  flow).
- Missing source files are silently skipped (no error, no warning) — each entry
  in `bonsai.copy` is copied on a best-effort basis.
- Configuration is read via `git config --get-all bonsai.copy`, reusing git's
  own config resolution (local repo config, global `~/.gitconfig`, etc.) instead
  of introducing a bonsai-specific config file or format.
- No new CLI flags; behavior is driven entirely by git config.

## Capabilities

### New Capabilities

- `worktree-copy-ignored-files`: Reading the `bonsai.copy` git config list and
  copying the named git-ignored files from the origin worktree into a
  provisioned/reused pool slot during `bs get`, skipping missing entries.

### Modified Capabilities

- `worktree-get`: `bs get` additionally copies configured ignored files into the
  provisioned slot as part of the provisioning flow, for both newly created and
  reused-existing slot paths.

## Impact

- Affected code: `src/worktree/mod.rs` (new config-reading and file-copy
  helpers, invoked from `get_worktree`), `src/main.rs` (no CLI changes expected,
  only wiring if needed).
- Affected config surface: git config gains a new namespaced key, `bonsai.copy`
  (repeatable), documented as part of `bs`'s configuration.
- No breaking changes; behavior is opt-in and inert when `bonsai.copy` is unset.
