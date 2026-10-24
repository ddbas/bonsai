## Context

See proposal.md for motivation. Classification lives in one function,
`classify(locked, dirty, has_processes)` in `src/worktree/mod.rs`, shared by
`bs list`, `bs status`, `bs get` (`find_available_slot`) and `bs prune`. Slots
are already inspected on parallel threads. `bs` never fetches, so every signal
must come from local refs.

## Goals / Non-Goals

**Goals:**

- Replace the process signal with a git-state signal in the same single
  classification path, evaluated cheapest-first with early return.
- Err toward `in use` whenever merged-ness cannot be proven.

**Non-Goals:**

- Network access (`ls-remote`, forge APIs) or fetching.
- Multi-remote / fork setups; a per-repo remote override may come later.
- Detecting active use by editors/agents (that is what `bs lock` is for).

## Decisions

1. **Ancestry first.** `git merge-base --is-ancestor HEAD D` (exit 0 ⇒ no
   unmerged work). One cheap call covering fresh branches, detached-at-merged
   and fast-forward/merge-commit merges. `rev-list --count D..HEAD` is used only
   for the "N commits ahead" reason in `bs status`.
2. **Rewritten merges via content checks**, only when ancestry fails and the
   branch is proven pushed and its remote branch gone:
   - Pushed proof: `branch.<b>.remote` and `.merge` configured (remote `.` does
     not count). Required: without it "remote ref absent" cannot be told apart
     from "never pushed". Alternative (accept absence alone) rejected as less
     strict; can be relaxed later.
   - Remote gone: `git rev-parse --verify -q refs/remotes/origin/<merge-branch>`
     fails. Local ref chosen over `ls-remote`: offline, fast, no credentials.
     Cost: stale until `fetch --prune`.
   - Content on D: `git cherry D HEAD` with no `+` lines (rebase), else
     `git merge-tree --write-tree D HEAD` exits 0 with result tree == `D^{tree}`
     (squash; git ≥ 2.38, otherwise `in use`).
3. **Default ref D** resolved once per invocation via
   `git symbolic-ref -q --short refs/remotes/origin/HEAD`, falling back to
   `origin/main`, `origin/master`, then local `main`/`master` only when no
   remote exists. Remote-tracking preferred because a local `main` may be stale
   or ahead. Unresolvable ⇒ `in use` with a reason.
4. **Remote is always `origin`.** Simplest; override deferred.
5. **Module and types.** New `src/worktree/branch_state.rs` with
   `resolve_default_ref()` and
   `branch_state(slot, branch, &DefaultRef) -> BranchState` (`NoOwnCommits`,
   `MergedRemoteGone`, `Unpushed(n)`, `RemoteExists`, `NotMerged(n)`,
   `DetachedWithCommits(n)`, `DefaultUnknown`) with `is_available()`.
   `classify(locked, dirty, has_unmerged_work)` stays the single decision point;
   the default ref is resolved once and passed to per-slot threads.
6. **No global warning for missing `push.autoSetupRemote`.** `bs status` shows a
   contextual reason instead, and the README carries the tip.

## Risks / Trade-offs

- Active work looks idle (clean slot, running editor) → documented; `bs lock`.
- Stale remote-tracking refs keep merged slots `in use` until
  `git fetch --prune` → safe direction; documented.
- `merge-tree` conflicts if D later edited the same lines → stays `in use`
  (safe); cannot produce a false "merged".
- Push without `-u` leaves no upstream config → rewritten-merge branches stay
  `in use`; reason text and README tip mitigate.
- Per-slot cost goes from one `lsof` to ≈1 `merge-base` (common) up to ~5 cheap
  git calls → rerun the `bs_ls` bench.
- Reset safety: `bs get` only moves HEAD; the local branch ref survives and all
  commits are on D, so reuse loses no data.

## Migration Plan

Behavior change only; no data migration. Remove `lsof` code and tests in the
same change; README note on the new semantics.
