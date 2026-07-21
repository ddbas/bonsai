## 1. Branch-ownership lookup

- [x] 1.1 Add a helper (e.g.
      `find_slot_checked_out_on_branch(pool_dir, branch) -> Result<Option<PathBuf>>`)
      in `src/worktree/mod.rs` that scans the pool directory's existing slots
      and returns the path of the first slot whose checked-out branch matches
      `branch`, reusing the same per-slot "which branch is checked out here"
      lookup already used by `list_worktrees_status`/`current_worktree`.
- [x] 1.2 Ensure the helper handles a non-existent `pool_dir` by returning
      `Ok(None)` (mirroring `find_slot_for_cwd`), and treats detached-HEAD slots
      as non-matches.
- [x] 1.3 Add unit tests for the helper covering: no pool dir yet, pool dir with
      no matching branch, pool dir with exactly one matching slot, pool dir with
      a matching slot that is locked.

## 2. Wire the lookup into `get_worktree`

- [x] 2.1 In `get_worktree`, after computing `pool_dir` (and
      creating/canonicalizing it) but before calling `find_available_slot`,
      check: if `branch` is `Some(BranchMode::Existing(name))`, call the new
      helper with `name`.
- [x] 2.2 If the helper returns `Some(existing_slot)`, short-circuit and return
      that canonicalized path immediately — skip `find_available_slot`,
      `reset_slot`, and `create_slot` entirely for this invocation.
- [x] 2.3 If the helper returns `None`, fall through to the existing
      provisioning flow unchanged (so non-existent branches and
      unmanaged-worktree conflicts still surface git's native error exactly as
      before).
- [x] 2.4 Confirm `BranchMode::New` and `BranchMode::Reset` paths are untouched
      by this change (no call to the new helper for those modes).

## 3. CLI output

- [x] 3.1 Verify `main.rs`'s existing `Some(Commands::Get { .. })`
      branch-name-in-output logic (`🌳 <path>  (<branch>)`) already produces the
      correct output for the short-circuited path with no changes needed; adjust
      only if the returned path/branch pairing needs re-deriving.

## 4. Tests

- [x] 4.1 Replace
      `get_positional_branch_already_checked_out_in_managed_slot_errors` in
      `tests/worktree_get.rs` with a test asserting `bs get shared-branch` exits
      0 and prints the already-claimed slot's path and branch name, without
      creating or resetting any other slot.
- [x] 4.2 Add a test where the already-claimed managed slot has been locked via
      `bs lock`; assert `bs get <branch>` still exits 0 and returns that slot's
      path, and the slot remains locked afterward.
- [x] 4.3 Confirm
      `get_positional_branch_already_checked_out_in_unmanaged_worktree_errors`
      still passes unmodified (unmanaged-worktree case remains an error).
- [x] 4.4 Confirm existing `-b`/`-B` "fails when branch already exists" /
      "succeeds when branch already exists" tests still pass unmodified.
- [x] 4.5 Add a regression test: calling `bs get shared-branch` twice in a row
      after the initial `-b` provisioning both return the same slot path and
      neither errors.

## 5. Documentation

- [x] 5.1 Update the `Get` subcommand doc comment in `src/main.rs` (and any
      relevant README/help text) to mention that a positional `<branch>` already
      checked out in one of this repo's managed slots is returned as-is rather
      than erroring.
