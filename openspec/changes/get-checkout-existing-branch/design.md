## Context

`bs get` currently accepts `-b <branch>` (create new branch, mirrors
`git checkout -b`) and `-B <branch>` (create-or-reset, mirrors
`git checkout -B`). Both are implemented via `worktree::BranchMode`
(`New`/`Reset`) and threaded through `reset_slot`/`create_slot` in
`src/worktree/mod.rs`. There is no path that checks out a branch that already
exists without creating or resetting it, and no check for "branch is already
checked out in another worktree" — `git checkout` inside a slot will happily
fail loudly for that case today, but only after a slot has been reset/allocated,
which is wasteful and can corrupt slot reuse bookkeeping.

Constraints:

- Must reuse the existing slot-provisioning pipeline (pool scan, prune,
  create-dir-all) unchanged.
- Must not require a second `git` invocation where one already suffices (mirrors
  existing single-subprocess `HEAD` + `git-common-dir` resolution).
- `clap` already encodes `-b`/`-B` as mutually exclusive `Option<String>`
  fields; the new positional argument must be added without breaking that.

## Goals / Non-Goals

**Goals:**

- Add a positional `[BRANCH]` argument to `bs get` that checks out an _existing_
  branch inside the provisioned/reused slot.
- Validate, before touching any slot, that (a) the branch exists in the repo and
  (b) it is not already checked out in another worktree (managed or not).
- Keep `-b`/`-B` behavior completely unchanged; make the three forms
  (positional, `-b`, `-B`) pairwise mutually exclusive via `clap`.

**Non-Goals:**

- No change to slot discovery, pruning, locking, or UUID naming.
- No support for checking out a branch that lives only on a remote (no implicit
  `git checkout --track origin/<branch>`); out of scope for this change.
- No change to `bs get` with no arguments (detached HEAD default) or to the
  no-subcommand default path.

## Decisions

- **New `BranchMode::Existing(String)` variant** rather than a separate
  function: keeps `reset_slot`/`create_slot` as the single place that maps a
  `BranchMode` to a `git checkout` invocation, avoiding duplicated
  slot-provisioning logic. `Existing` maps to `git -C <slot> checkout <branch>`
  (no `-b`/`-B` flag), matching `git checkout <existing-branch>` semantics
  exactly.
- **Pre-flight branch checks happen before slot provisioning.** Before
  scanning/creating a slot, `get_worktree` will, when `BranchMode::Existing` is
  passed:
  1. Verify the branch exists via
     `git show-ref --verify --quiet refs/heads/<branch>` (single lightweight
     call, no working-tree mutation).
  2. Verify the branch is not already the target of another worktree via
     `git worktree list --porcelain` (already parsed elsewhere for pruning;
     reuse the parsing helper) — if a `branch refs/heads/<branch>` line appears
     for any worktree, error out immediately, mirroring `git worktree    add`'s
     native "already checked out" failure. This ordering avoids
     resetting/creating a slot only to fail the `git checkout` step afterward,
     which would leave a slot in a half-provisioned state.
- **`clap` mutual exclusion via `conflicts_with` on all three fields.** The
  positional `branch: Option<String>` argument gets
  `conflicts_with = "new_branch"` and `conflicts_with = "reset_branch"`;
  `new_branch` and `reset_branch` already conflict with each other. This gives
  pairwise exclusion without custom validation code.
- **Error messages name the conflicting branch/worktree path**, consistent with
  existing `-b` "branch already exists" error, so users can immediately `bs get`
  into the existing worktree or pick a different branch.

## Risks / Trade-offs

- [Risk] Reusing `git worktree list --porcelain` parsing for the
  already-checked-out check duplicates a small amount of logic already used for
  stale-registration pruning → Mitigation: extract/reuse the existing parsing
  helper rather than re-implementing porcelain parsing.
- [Risk] A branch could be deleted or checked out elsewhere between the
  pre-flight check and the actual `git checkout` inside the slot (race
  condition) → Mitigation: accept as a known limitation (matches
  `git worktree add`'s own non-atomic behavior); surface the underlying `git`
  error if the final checkout still fails.
- [Trade-off] Adding a positional argument to a subcommand that also has two
  optional flags increases `clap` complexity slightly, but keeps the CLI surface
  small (`bs get <branch>` reads naturally, avoiding a new `--checkout`/`-c`
  flag).

## Open Questions

None outstanding; behavior fully specified by the mutual-exclusion and
already-checked-out error requirements above.
