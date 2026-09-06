## Context

Bonsai manages a pool of git worktrees per repository under
`managed_root()/repo_slug()`. `git worktree` already provides all the primitives
for registering and deregistering worktrees:

- `git worktree list --porcelain` enumerates registered worktrees, their lock
  state, and their branch (already wrapped by `worktree::list_pool_worktrees` /
  `list_pool_worktrees_checking_stale`).
- `git worktree prune` deregisters any worktree whose administrative files point
  at a directory that no longer exists on disk. Crucially, it does **not**
  delete anything itself — it only cleans up bookkeeping for directories that
  are already gone.

Today nothing in bonsai ever deletes a slot's directory, so `git worktree prune`
currently has nothing to do; stale slots just accumulate. `bs list` /
`bs status` already classify every slot as `available`, `in use`, or `locked`
via `worktree::classify_slot_status` (locked > in use > available), and
`bs list`'s `list_worktrees_status` already computes this classification for
every pool slot concurrently.

## Goals / Non-Goals

**Goals:**

- Add `bs prune`, which deletes the on-disk directories of slots classified
  `available` and then calls `git worktree prune` so git's own bookkeeping
  catches up.
- Reuse the existing classification/enumeration code (`list_worktrees_status`,
  `classify_slot_status`, `list_pool_worktrees`) rather than writing new
  git-porcelain parsing.
- Print per-pruned-slot identifying info (tilde path + branch), matching the
  style already used by `bs list`/`bs get`.
- Be conservative: never touch `in use` or `locked` slots, never touch anything
  outside the current repository's pool directory.

**Non-Goals:**

- Reimplementing any part of `git worktree remove`/`git worktree prune` (e.g.
  manually editing `.git/worktrees/*`, manually deleting branches, or manually
  deleting refs). All git-level deregistration is delegated to
  `git worktree prune`.
- Pruning slots that are merely stale/unreachable on the git side but whose
  directory still exists — `bs prune` only ever removes a directory when bonsai
  itself has classified the slot `available`; it is not a generic wrapper around
  `git worktree prune` for arbitrary non-pool worktrees.
- Any interactive confirmation prompt (the "available" classification is already
  the same conservative bar `bs get` uses to decide what it's safe to
  reuse/reset).

## Decisions

1. **Classification reuse**: `bs prune` calls
   `worktree::list_worktrees_status(&pool_dir)` (same function `bs list` uses)
   to get `(path, status, branch)` for every pool slot, then filters to
   `WorktreeStatus::Available`. This guarantees `bs prune` and `bs list` can
   never disagree about which slots are "available" — one classification
   function, two consumers.

   _Alternative considered_: write a bespoke "is this slot deletable" check
   inside the prune command. Rejected — would duplicate the exact
   locked/dirty/open-files priority logic already centralized in
   `classify_slot_status`/`classify`, risking drift between `bs list` and
   `bs prune`.

2. **Deletion is `std::fs::remove_dir_all` only, no git surgery**: for each
   available slot, `bs prune` calls `std::fs::remove_dir_all(&path)` on the slot
   directory and nothing else. It does not touch `.git/worktrees/*`, does not
   run `git worktree remove`, and does not delete branches. All of that is left
   to the subsequent `git worktree prune` call.

   _Alternative considered_: run `git worktree remove <path>` per slot instead
   of deleting the directory ourselves, skipping the final `git worktree prune`
   call entirely. Rejected — the proposal explicitly asks to lean on
   `git worktree prune` for cleanup and to keep bonsai's own responsibility
   limited to "delete the folders we manage"; `git worktree remove` also refuses
   on some edge cases (e.g. locked, or the directory already partially gone) in
   ways that would need separate error handling, whereas plain directory
   removal + `git worktree prune` is simpler and composes with git's own
   idempotent cleanup.

3. **Single final `git worktree prune` call, not one per slot**: after deleting
   all available slots' directories, `bs prune` runs `git worktree prune`
   exactly once (via the existing `git_cmd()` helper), matching how
   `git worktree prune` is designed to be used (it scans and cleans up all stale
   entries in one pass).

   _Alternative considered_: run `git worktree prune` after each individual
   directory deletion. Rejected — unnecessary process spawns; one pass at the
   end is sufficient and matches idiomatic `git worktree prune` usage.

4. **Output format**: reuse `worktree::tilde_path` for the path and the same
   `path (branch)` bold-parenthesis formatting already used by `bs list`, so
   `bs prune`'s output is visually consistent with the rest of the CLI. The list
   of pruned slots is captured _before_ deletion (from `list_worktrees_status`)
   so it can be printed regardless of ordering effects from
   `git worktree prune`.

5. **No pool / nothing available**: mirror `bs list`'s existing friendly "No
   worktrees managed..." message pattern for the "pool directory doesn't exist"
   case, and add a distinct "nothing to prune" message when the pool exists but
   no slot is `available`. In both cases `bs prune` still runs
   `git worktree prune` at the end (a no-op if there's nothing stale), keeping
   behavior predictable, and exits 0.

## Risks / Trade-offs

- **[Risk] Accidental deletion of a slot that is actually in use due to a
  classification race (e.g. a process opens a file in the slot right after
  `bs prune` classified it as available)** → Mitigation: same race already
  exists for `bs get`'s reuse logic; classification happens immediately before
  deletion (no unbounded delay between check and delete), and the existing
  `classify_slot_status` priority rules (locked > in use > available) are
  unchanged, so `bs prune`'s risk profile matches `bs get`'s existing, accepted
  risk profile.
- **[Risk] `remove_dir_all` failing partway through (e.g. permissions, open file
  handle on some platforms) leaves a partially-deleted directory** → Mitigation:
  treat per-slot deletion failures as non-fatal — log/report the error for that
  slot, continue pruning other available slots, and still run
  `git worktree prune` at the end so any slots that _did_ fully delete are
  cleaned up; surface a non-zero exit only if at least one deletion failed,
  matching the "best effort, report what happened" tone of
  `bs list`/`bs status`.
- **[Risk] Deleting something outside the managed pool** → Mitigation:
  `bs prune` only ever operates on paths returned by
  `list_pool_worktrees`/`list_worktrees_status`, which already scope entries to
  `path.starts_with(&pool_canonical)`; no user-supplied path is accepted by
  `bs prune` (unlike `bs lock`/`bs status`).

## Migration Plan

Purely additive: a new subcommand with no changes to existing commands' behavior
or data formats. No migration or rollback steps beyond normal
release/versioning; no persisted state changes.

## Open Questions

- Should `bs prune` accept a `--dry-run` flag to preview what would be deleted
  without deleting? Not required by the proposal; can be added later without
  breaking the default behavior. Left out of this change to keep scope minimal.
