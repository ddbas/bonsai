## Why

`bs get [branch]` currently fails when the requested branch is already checked
out in another bonsai-managed pool worktree, forcing the caller to manually
locate and `cd` into that existing worktree themselves. Since bonsai already
owns that worktree, the tool should recognize this case and hand the caller the
existing slot instead of erroring out.

## What Changes

- `bs get <branch>` SHALL, before attempting to provision/reset a slot, check
  whether `<branch>` is already checked out in one of **this repo's**
  bonsai-managed pool worktrees.
- If a bonsai-managed slot already has `<branch>` checked out, `bs get <branch>`
  SHALL print that existing slot's path (with the branch name) and exit 0,
  without provisioning a new slot, resetting any slot, or invoking
  `git checkout`/`git worktree add` for the branch.
- If the branch does not exist, or is checked out in an **unmanaged** worktree
  (outside the bonsai pool), behavior is unchanged: `bs get <branch>` still
  fails and surfaces git's own error.
- This does not apply to `-b`/`-B`: those flags keep their existing "fail if
  exists" / "create-or-reset" semantics respectively, since they are requests to
  create or reset a branch, not to reuse whatever slot currently holds it.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `get-checkout-existing-branch`: the requirement "Branch already checked out
  elsewhere is rejected" changes for the managed-slot case: instead of erroring,
  `bs get <branch>` now detects that the branch is already checked out in a
  bonsai-managed slot for this repo and returns that slot's path successfully.
  The unmanaged-worktree case is unchanged and still errors.

## Impact

- `src/worktree/mod.rs`: `get_worktree` gains a lookup step (using
  `list_worktrees_status`/pool scanning) to detect an existing managed slot
  already checked out on the requested branch, for `BranchMode::Existing` only.
- `tests/worktree_get.rs`: the existing test
  `get_positional_branch_already_checked_out_in_managed_slot_errors` needs to be
  replaced with a test asserting the existing managed slot is returned
  successfully instead of erroring.
- No CLI flag or output format changes beyond exit code and message content for
  this one scenario.
