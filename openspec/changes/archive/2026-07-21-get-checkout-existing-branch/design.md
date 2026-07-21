## Context

`bs get` currently accepts `-b <branch>` (create new branch, mirrors
`git checkout -b`) and `-B <branch>` (create-or-reset, mirrors
`git checkout -B`). Both are implemented via `worktree::BranchMode`
(`New`/`Reset`) and threaded through `reset_slot`/`create_slot` in
`src/worktree/mod.rs`. There is no path that checks out a branch that already
exists without creating or resetting it.

Critically, both `git checkout <branch>` and `git worktree add <path> <branch>`
already refuse to operate when `<branch>` does not exist ("error: pathspec
'<branch>' did not match any file(s) known to git" / "fatal: invalid reference")
or when `<branch>` is already checked out in another worktree ("fatal:
'<branch>' is already used by worktree at '<path>'") — this worktree-aware guard
has been part of git's own checkout and worktree-add code paths since the
`git worktree` feature was introduced in git 2.5, independent of the bonsai
pool. There is therefore no need for `bs get` to reimplement these checks
itself; it can simply run the plain git command and propagate the failure.

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
- Surface git's own "branch does not exist" and "branch already checked out
  elsewhere" failures as actionable `bs get` errors, without reimplementing
  those checks in the CLI.
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
  `BranchMode` to a git invocation, avoiding duplicated slot-provisioning logic.
  - In `reset_slot` (reusing an available slot), `Existing` maps to
    `git -C <slot> checkout <branch>` — no `head_sha` argument, since the point
    is to land on the branch's own tip rather than force it to the caller's
    HEAD, and no `-b`/`-B` flag, matching plain `git checkout <existing-branch>`
    semantics.
  - In `create_slot` (no available slot found), `Existing` maps to
    `git worktree add <slot_path> <branch>` — deliberately omitting `--detach`
    and `head_sha`, mirroring plain `git worktree add <path> <branch>` for an
    existing branch.
- **No custom pre-flight validation.** `bs get <branch>` does **not** run any
  extra `git show-ref` / `git worktree list --porcelain` checks before
  provisioning. It relies entirely on the exit status and stderr of the
  `git checkout` / `git worktree add` call already made in `reset_slot` /
  `create_slot`: if the branch doesn't exist or is already checked out
  elsewhere, that call fails and `bs get` propagates the failure (wrapping git's
  stderr in the existing `bail!("...: {stderr}")` pattern used elsewhere in this
  module). This avoids two extra git subprocesses per invocation and avoids
  duplicating validation logic that git already maintains correctly (including
  edge cases like detached-HEAD worktrees or branches checked out via
  `git switch`).
- **`clap` mutual exclusion via `conflicts_with` on all three fields.** The
  positional `branch: Option<String>` argument gets
  `conflicts_with = "new_branch"` and `conflicts_with = "reset_branch"`;
  `new_branch` and `reset_branch` already conflict with each other. This gives
  pairwise exclusion without custom validation code.
- **Error messages are git's own stderr**, passed through unchanged (as
  `reset_slot`/`create_slot` already do for the `-b`/`-B` paths). Git's built-in
  messages already name the offending branch or the conflicting worktree path,
  so no extra formatting is needed to make them actionable.

## Risks / Trade-offs

- [Risk] For the reuse path, `reset_slot`'s `git checkout <branch>` failing
  leaves the slot untouched (still clean and unlocked) rather than reset to HEAD
  → Mitigation: this is acceptable — the slot remains available for a future
  `bs get` call, and matches the existing behavior when `-b`/`-B` checkout
  fails.
- [Risk] For the no-available-slot path, a failing `git worktree add` generally
  does not register a worktree or leave the target directory populated →
  Mitigation: no cleanup logic is added for this change; if git ever leaves a
  partial directory behind, it is indistinguishable from any other
  `git worktree add` failure already possible today (e.g. `-b` with a colliding
  branch name) and is out of scope here.
- [Trade-off] Error messages are exactly what git prints, not a bonsai-authored
  message → acceptable, since git's messages are already specific and actionable
  (they name the branch and, for the worktree conflict, the exact path), and
  rewriting them would risk drifting from git's actual behavior across git
  versions.
- [Trade-off] Adding a positional argument to a subcommand that also has two
  optional flags increases `clap` complexity slightly, but keeps the CLI surface
  small (`bs get <branch>` reads naturally, avoiding a new `--checkout`/`-c`
  flag).

## Open Questions

None outstanding; behavior fully specified by the mutual-exclusion requirement
and by delegating existence/already-checked-out validation to the underlying git
commands.
