## Why

Over time, bonsai pool slots that are no longer needed pile up on disk with no
way to reclaim them: `bs get` only reuses a slot's git worktree registration,
and stale registrations are only cleaned up by `git worktree prune` — which
itself does nothing until the slot's directory is already gone. Users currently
have to manually `rm -rf` slot directories and then remember to run
`git worktree prune`. A `bs prune` command should automate the safe half of that
workflow (deleting only the folders that are actually unused) and delegate the
git-level bookkeeping to `git worktree prune` itself, rather than reimplementing
worktree removal.

## What Changes

- Add a new `prune` subcommand to the CLI.
- `bs prune` SHALL, for the current repository's pool:
  1. Classify every managed pool slot using the existing `available` / `in use`
     / `locked` classification (same rules as `bs list`/`bs status`).
  2. Delete the on-disk directory for every slot classified `available` (an
     `in use` or `locked` slot is never touched or deleted).
  3. Run `git worktree prune` so git deregisters the worktrees whose directories
     were just deleted.
- `bs prune` SHALL NOT reimplement any worktree removal/deregistration logic
  itself (e.g. no manual editing of `.git/worktrees/*`, no manual branch
  deletion) — that responsibility is left entirely to `git worktree prune`.
- `bs prune` SHALL print, for each slot that was pruned, the same identifying
  information already used elsewhere (tilde-abbreviated path and checked-out
  branch, if any), so the user can see exactly what was removed.
- If there is nothing to prune (no pool, or no available slots), `bs prune`
  SHALL print a clear message and exit successfully instead of silently doing
  nothing.

## Capabilities

### New Capabilities

- `worktree-prune`: A `bs prune` subcommand that deletes the directories of
  available (unused, unlocked) bonsai pool slots and then runs
  `git worktree prune` to let git deregister them, printing the identifying info
  of each pruned slot.

### Modified Capabilities

(none — this introduces a new subcommand without changing the requirements of
existing commands)

## Impact

- **Affected code**: `src/main.rs` (new `Commands::Prune` variant + handler),
  `src/worktree/mod.rs` (new helper(s) to enumerate available slots and delete
  their directories, reusing `list_worktrees_status`/ `classify_slot_status`,
  `tilde_path`, and `managed_root`/`repo_slug`).
- **Affected tests**: new `tests/worktree_prune.rs` integration test, following
  the conventions of `tests/worktree_list.rs` / `tests/worktree_lock.rs`.
- **Dependencies**: relies on the system `git` binary already required by the
  rest of bonsai; no new external dependencies.
- **Risk**: irreversible deletion of directories — must be conservative and only
  ever touch slots classified strictly `available` (never `in use` or `locked`),
  and never delete anything outside the managed pool directory for the current
  repository.
