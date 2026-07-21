## Context

`bs get` provisions or reuses a slot from a per-repo pool under
`~/.bonsai/<repo-slug>/`. When called with a positional `<branch>` argument
(`BranchMode::Existing`), it currently always calls `find_available_slot` (which
only considers slots that are free/unlocked/clean) and then attempts
`git checkout <branch>` (reuse path) or `git worktree add <slot> <branch>`
(new-slot path). Git itself refuses to check out a branch that is already
checked out in _any_ worktree (managed or not), so today `bs get <branch>` fails
whenever `<branch>` is checked out anywhere — including in a slot that bonsai
itself manages for this repo.

That failure is unhelpful in the common case: a teammate (or an earlier
invocation) already ran `bs get -b <branch>` or `bs get <branch>`, bonsai
already has a slot sitting on that branch, and the caller just wants that slot's
path. Forcing an error means the caller must run `bs list`/`bs current` or dig
through `~/.bonsai/...` by hand.

`list_worktrees_status(pool_dir)` already exists and returns, per pool slot, its
path, lock/availability status, stats, and checked-out branch name (via
`git branch --show-current` or similar per-slot lookup) — this is the same data
source `bs list` renders. That is exactly the lookup needed to detect "this
branch is already checked out in a slot I manage."

## Goals / Non-Goals

**Goals:**

- When `bs get <branch>` (positional / `BranchMode::Existing` only) is asked for
  a branch that is already checked out in one of this repo's bonsai-managed pool
  slots, return that slot's path (with branch name in the printed output) and
  exit 0 — no new slot, no reset, no `git checkout`/`git worktree add`
  invocation for the branch.
- Preserve all existing behavior for: branch does not exist anywhere; branch
  checked out in an _unmanaged_ worktree; `-b`/`-B` modes; `bs get` with no
  branch.
- Keep the fix scoped to `get_worktree`/pool-scanning logic; no CLI flag
  changes.

**Non-Goals:**

- Do not change `-b`/`-B` semantics. `-b` must still fail if the branch exists
  anywhere; `-B` must still create-or-reset. Neither should silently redirect to
  an existing slot, because they express an explicit intent to create/reset a
  branch, not to "get me whatever slot has this branch." Reusing an
  already-locked slot elsewhere would also make `-B`'s "reset to HEAD" semantics
  ambiguous (reset which slot?).
- Do not add a new subcommand or flag to opt in/out of this behavior.
- Do not change detection for unmanaged worktrees; those still surface git's
  native error.

## Decisions

**Decision: detect via a pre-check against pool slots, scoped to
`BranchMode::Existing`, before calling `find_available_slot`/`reset_slot`/
`create_slot`.**

In `get_worktree`, after resolving `head_sha`/`slug`/`pool_dir` but before
scanning for an available slot, add a branch-ownership lookup: if
`branch == Some(BranchMode::Existing(name))`, scan the pool directory's existing
worktrees (reusing the same per-slot "what branch is checked out here" logic
that backs `list_worktrees_status`/`current_worktree`) for a slot already on
`name`. If found, return that slot's canonicalized path immediately, skipping
`find_available_slot`, `reset_slot`, and `create_slot` entirely.

Alternatives considered:

- _Let git fail, then parse stderr to detect "already checked out" and recover._
  Rejected: fragile (depends on git's exact error wording/locale), and by the
  time git fails, `find_available_slot` may already have picked a _different_
  slot to reset/create, so recovery would need to undo that work. A pre-check
  keeps `get_worktree` a straight sequential provisioning flow with no undo
  logic.
- _Apply the same "return existing" logic to `-b`._ Rejected: `-b`'s whole
  contract is "fail if the branch exists" (mirrors `git checkout -b`); silently
  redirecting would violate that documented contract and existing tests.
- _Apply it to `-B`._ Rejected: `-B` means "create-or-reset the branch at HEAD";
  if the branch already lives in another slot, resetting _that_ slot's branch
  while the caller is standing in a different directory is surprising, and the
  target slot for the reset would be ambiguous (the found slot vs. a freshly
  provisioned one). Left for a future change if ever needed.

**Decision: only match against slots for this repo's pool (this repo's
`pool_dir`), not other repos' pools.**

Branch names are only meaningfully unique within one repo, and only this repo's
`pool_dir` is already computed by `get_worktree`. No cross-repo scanning is
introduced.

**Decision: unmanaged worktrees remain untouched — the check only inspects
bonsai's own pool slots.**

The proposal explicitly keeps unmanaged-worktree conflicts erroring via git's
native message, since bonsai has no path to safely hand back a worktree it
doesn't own/manage.

## Risks / Trade-offs

- [Risk] A pool slot could be _locked_ while checked out on the requested branch
  (e.g. someone ran `bs lock` on it). Returning a locked slot's path is still
  correct here — locking prevents `bs get`/`bs list` from _reusing_ it for a
  _different_ purpose, but the caller explicitly asked for the branch that slot
  already holds, so returning its path is the desired behavior, not a bypass of
  the lock. → No mitigation needed; document the behavior in the spec scenario.
- [Risk] The per-slot "which branch is checked out here" lookup used by
  `list_worktrees_status` shells out to git per slot; adding this check to
  `get_worktree`'s hot path adds O(slots) git calls only when a positional
  branch argument is supplied (existing `bs get`/`-b`/`-B` invocations are
  unaffected). → Acceptable: bounded by pool size per repo, and only paid when a
  positional branch is actually given.
- [Risk] Existing test
  `get_positional_branch_already_checked_out_in_managed_slot_errors` encodes
  today's "error" behavior and will fail once fixed. → Update/replace that test
  as part of this change (tracked in tasks.md) rather than leaving it red.

## Migration Plan

No data migration. This is a pure behavior change in `bs get <branch>` for one
previously-erroring case. Ship as a normal patch release; no rollback concerns
beyond reverting the code change if regressions surface.

## Open Questions

None outstanding — behavior for `-b`/`-B` and unmanaged worktrees is explicitly
out of scope per the Non-Goals above.
