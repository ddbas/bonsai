## 1. Branch mode & CLI wiring

- [ ] 1.1 Add `BranchMode::Existing(String)` variant in `src/worktree/mod.rs`
- [ ] 1.2 Add positional `branch: Option<String>` argument to `Commands::Get` in
      `src/main.rs`, with `conflicts_with = "new_branch"` and
      `conflicts_with = "reset_branch"`
- [ ] 1.3 Wire the positional argument into `BranchMode::Existing` and pass it
      to `get_worktree`, matching the existing `new_branch`/`reset_branch`
      dispatch pattern

## 2. Pre-flight branch validation

- [ ] 2.1 Add a helper to check branch existence via
      `git show-ref --verify     --quiet refs/heads/<branch>` (or equivalent)
      without mutating any worktree
- [ ] 2.2 Reuse/extract the existing `git worktree list --porcelain` parsing
      helper to detect whether `<branch>` is already checked out in any worktree
      (managed or unmanaged), returning that worktree's path when found
- [ ] 2.3 In `get_worktree`, when `branch` is `BranchMode::Existing`, run both
      checks **before** pool scanning/slot creation; bail out with an actionable
      error (naming the branch, or the conflicting worktree path) if either
      check fails

## 3. Slot checkout logic

- [ ] 3.1 Extend `reset_slot` and `create_slot` to handle `BranchMode::Existing`
      by running `git -C <slot> checkout <branch>` (no `-b`/`-B`)
- [ ] 3.2 Ensure the branch name is surfaced back to `main.rs` for output
      formatting, matching the `-b`/`-B` output path (`🌳 <path>  (<branch>)`)

## 4. Tests

- [ ] 4.1 Unit test: `bs get <branch>` on an existing, unclaimed branch checks
      out that branch in the slot (not detached)
- [ ] 4.2 Unit test: `bs get <branch>` on a non-existent branch errors out
      without creating/resetting any slot
- [ ] 4.3 Unit test: `bs get <branch>` on a branch already checked out in
      another managed slot errors out and names the conflicting path
- [ ] 4.4 Unit test: `bs get <branch>` on a branch already checked out in an
      unmanaged worktree errors out and names the conflicting path
- [ ] 4.5 CLI parsing test: positional `branch` conflicts with `-b`
- [ ] 4.6 CLI parsing test: positional `branch` conflicts with `-B`
- [ ] 4.7 Update/extend existing `-b`/`-B` conflict tests to confirm they are
      unaffected by the new positional argument

## 5. Documentation

- [ ] 5.1 Update `bs get` help text / doc comments in `src/main.rs` to describe
      the new positional `<branch>` argument and its interaction with `-b`/`-B`
