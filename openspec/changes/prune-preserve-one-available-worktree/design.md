## Context

`bs prune` (see `openspec/specs/worktree-prune/spec.md`) currently deletes the
on-disk directory of _every_ slot `list_worktrees_status` classifies
`Available`, then runs `git worktree prune` once to let git deregister them.
This is implemented by `prune_available_slots` / `prune_pool` in
`src/worktree/mod.rs`.

The problem: bonsai's core promise is a warm pool — `bs get` should usually find
an existing available slot instead of paying the cost of `git worktree add`. If
`bs prune` empties the pool, the very next `bs get` loses that benefit.
`bs prune` must always leave at least one available slot on disk and registered,
even if that means detaching a branch from it instead of deleting it.

`reset_slot(slot_path, head_sha, None)` already implements "detach a slot to a
given SHA" (`git checkout --detach <head_sha>`) — it's the exact mechanism
`bs get` uses when handing out a slot with no branch requested. `resolve_head()`
gives the SHA to detach to (current repo HEAD).

## Goals / Non-Goals

**Goals:**

- `bs prune` never deletes the last available slot; at least one available slot
  always remains, on disk and registered, after `bs prune` completes.
- When the preserved slot has a branch checked out, `bs prune` detaches it
  (reusing `reset_slot`) so it's immediately reusable, matching the invariant
  that available slots are ready-to-go.
- Deterministic, easy-to-explain selection of which slot is preserved.
- CLI reporting clearly distinguishes "deleted" slots from the "preserved
  (detached)" slot so users aren't confused about why one remains.

**Non-Goals:**

- No change to `bs get`, `bs list`, `bs status`, or the `locked`/`in use`
  classification rules.
- No configurability of the preserved-slot count (always exactly 1 when
  available slots exist); a future change could make this configurable.
- No change to how `git worktree prune` deregistration itself works.

## Decisions

### Decision: Select the slot to preserve deterministically by pool order

`prune_available_slots` iterates `list_worktrees_status`'s output, which already
preserves `git worktree list --porcelain` order (see the comment on
`list_worktrees_status`: results are joined "in original order"). We preserve
whichever available slot appears **first** in that order, and delete the rest.

Alternatives considered:

- Preserve the _last_ available slot: equally deterministic, but "first" reads
  naturally as "keep the earliest-registered/most-stable slot" and requires no
  extra reasoning about list reversal.
- Pick "most recently used" or "least recently used" slot via mtime/log: more
  surprising, adds filesystem-timestamp dependence and complexity for no clear
  benefit — any available slot is equally valid to keep since it will be
  detached anyway.

We choose first-in-list for simplicity and determinism; it needs no new metadata
and is trivial to test.

### Decision: Detach in place instead of "no-op" when the preserved slot has a branch

If the preserved available slot currently has a branch checked out, we call
`reset_slot(path, &resolve_head()?, None)` to detach it to current HEAD,
matching the existing "hand out a detached slot" code path used elsewhere. This
keeps the invariant that all _available_ slots are detached HEAD, ready for
reuse without carrying stale branch state forward.

If the preserved slot is already in detached HEAD, no git operation is performed
on it at all — it's simply excluded from the deletion candidate list.

Alternatives considered:

- Leave the branch attached on the preserved slot: rejected — this leaves a
  “used” branch dangling in a slot that `bs list`/`bs get` will still call
  available, which is confusing and inconsistent with what "available" otherwise
  means when handed out fresh.
- Delete the preserved slot's directory and recreate it via
  `git worktree add --detach`: unnecessarily expensive (full worktree
  re-checkout) compared to reusing the existing directory with a
  `checkout --detach` reset, when the directory is already exactly what's
  needed.

### Decision: Extend `PruneOutcome` with a `preserved: Option<PrunedSlot>` field

`prune_pool` returns a `PruneOutcome { pruned, failures }` today. We add a third
field, `preserved: Option<PrunedSlot>` (reusing the existing
`PrunedSlot { path, branch }` shape), set when there was at least one available
slot. `branch` on the returned `PrunedSlot` reflects the branch _before_
detaching (so the CLI can still say "detached `my-feature`"), even though the
slot itself has now been reset to detached HEAD.

This keeps the "what to report" data on the `PruneOutcome` value rather than
requiring the CLI layer to re-derive it, consistent with how `pruned` already
works.

Alternatives considered:

- Report the preserved slot as just another `pruned` entry with a flag: more
  intrusive to the existing struct semantics ("pruned" implies deletion); a
  separate field is clearer.

### Decision: Detach failure treated the same as a deletion failure

If detaching the preserved slot's branch fails (e.g. `git checkout --detach`
errors), `prune_pool` records it as a failure (added to `PruneOutcome`, e.g. a
new `preserve_failure: Option<(PathBuf, String)>` or reusing `failures` with a
distinguishing marker) and still proceeds to run `git worktree prune` and return
a non-zero-status-worthy outcome, mirroring the existing per-slot deletion
failure handling. The slot is _not_ deleted in this case (we never delete the
preserved slot, regardless of detach outcome) — it's simply left with its branch
still attached, and the CLI surfaces the failure.

## Risks / Trade-offs

- [Risk] Selecting "first in list" could always preserve the same physical slot
  across repeated `bs prune` runs, meaning that slot never gets
  reclaimed/rotated even if it becomes large/stale. → Mitigation: acceptable —
  the goal is _a_ warm slot, not disk reclamation of that specific slot; a
  future `bs prune --all`/force flag could allow opting out of preservation if a
  user really wants an empty pool.
- [Risk] Detaching a branch the user still cares about (even though it's
  classified `available`, i.e. clean/unlocked/no open files) could feel
  surprising if they expected `bs prune` to be a read-mostly/deletion-only
  operation on directories. → Mitigation: this matches existing `bs get`
  behavior on other available slots (branches on available slots are always
  liable to be detached when reused) and is exactly the deletion-avoidance
  behavior requested; document clearly in CLI output and spec scenarios.
- [Risk] Zero available slots: no behavior change, but must ensure the "keep
  one" logic doesn't misfire (e.g. attempt to detach a non-existent slot). →
  Mitigation: guard explicitly with a unit test for the empty-candidates case,
  and one for the exactly-one-candidate case.
