## 1. Branch mode & CLI wiring

- [x] 1.1 Add `BranchMode::Existing(String)` variant in `src/worktree/mod.rs`
- [x] 1.2 Add positional `branch: Option<String>` argument to `Commands::Get` in
      `src/main.rs`, with `conflicts_with = "new_branch"` and
      `conflicts_with = "reset_branch"`
- [x] 1.3 Wire the positional argument into `BranchMode::Existing` and pass it
      to `get_worktree`, matching the existing `new_branch`/`reset_branch`
      dispatch pattern

## 2. Slot checkout logic

- [x] 2.1 Extend `reset_slot` to handle `BranchMode::Existing` by running
      `git -C <slot> checkout <branch>` (no `head_sha`, no `-b`/`-B`) — rely on
      git's own failure when the branch doesn't exist or is already checked out
      elsewhere; do not add any pre-flight validation
- [x] 2.2 Extend `create_slot` to handle `BranchMode::Existing` by running
      `git worktree add <slot_path> <branch>` (no `--detach`, no `head_sha`, no
      `-b`/`-B`) — same reliance on git's native checks
- [x] 2.3 Ensure `get_worktree` propagates the underlying `git` stderr unchanged
      on failure (matching the existing `bail!` pattern used for `-b`/`-B`), so
      no slot is left half-provisioned
- [x] 2.4 Ensure the branch name is surfaced back to `main.rs` for output
      formatting, matching the `-b`/`-B` output path (`🌳 <path>  (<branch>)`)

## 3. Tests

- [x] 3.1 Integration test: `bs get <branch>` on an existing, unclaimed branch
      checks out that branch in the slot (not detached)
- [x] 3.2 Integration test: `bs get <branch>` on a non-existent branch errors
      out (relying on git's native error) without creating/resetting any slot
- [x] 3.3 Integration test: `bs get <branch>` on a branch already checked out in
      another managed slot errors out (relying on git's native error) and the
      error names the conflicting path
- [x] 3.4 Integration test: `bs get <branch>` on a branch already checked out in
      an unmanaged worktree errors out and the error names the conflicting path
- [x] 3.5 CLI parsing test: positional `branch` conflicts with `-b`
- [x] 3.6 CLI parsing test: positional `branch` conflicts with `-B`
- [x] 3.7 Update/extend existing `-b`/`-B` conflict tests to confirm they are
      unaffected by the new positional argument

## 4. Documentation

- [x] 4.1 Update `bs get` help text / doc comments in `src/main.rs` to describe
      the new positional `<branch>` argument and its interaction with `-b`/`-B`
      </content>
