## Why

Bonsai's whole value proposition is keeping a warm pool of worktrees so the
_next_ `bs get` is fast. `bs prune` currently deletes the directory of every
slot classified `available`, which can empty the pool entirely. The next
`bs get` then has to create a brand-new worktree from scratch, defeating the
purpose of the pool. `bs prune` must always leave at least one available slot
behind so the pool stays "warm."

## What Changes

- `bs prune` SHALL preserve at least one `available` slot's on-disk directory
  and worktree registration; it must never delete every available slot.
- When one or more available slots have a branch checked out, the preserved slot
  SHALL have any checked-out branch detached (reset to detached HEAD) so it is
  fully free for reuse, instead of leaving a stale branch attached.
- If there are zero or one available slots to begin with, behavior is unchanged
  except that a lone available slot is now detached (if it has a branch checked
  out) rather than deleted.
- If there are no available slots at all, `bs prune` behaves as today (nothing
  to preserve, nothing to delete).
- The prune summary output SHALL distinguish the preserved/detached slot from
  the deleted slots so the user understands why one available slot remains.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `worktree-prune`: adds a requirement that `bs prune` must always retain at
  least one available pool slot (detaching its branch instead of deleting it
  when multiple available slots exist), rather than deleting every available
  slot.

## Impact

- `src/worktree/mod.rs`: `prune_available_slots`, `prune_pool`, and related
  helpers need to select and reserve one available slot for detach-in-place
  instead of deletion, and reuse `reset_slot` (detach) for that slot.
- CLI output in `src/main.rs` (or wherever `bs prune` reporting lives) needs to
  report the preserved slot distinctly from deleted ones.
- Existing `worktree-prune` spec requirements and tests describing "delete all
  available slots" need to be updated to reflect the "delete all but one"
  behavior.
