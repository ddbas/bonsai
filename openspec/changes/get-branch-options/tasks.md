## 1. Library — BranchMode type and worktree helpers

- [ ] 1.1 Add `BranchMode` enum (`New(String)` / `Reset(String)`) to
      `src/worktree/mod.rs` with `pub` visibility
- [ ] 1.2 Extend `create_slot(slot_path, head_sha, branch: Option<&BranchMode>)`
      to pass `-b <name>` or `-B <name>` to `git worktree add` instead of
      `--detach` when `branch` is `Some`; `None` behaviour unchanged
- [ ] 1.3 Extend `reset_slot(slot_path, head_sha, branch: Option<&BranchMode>)`
      to run `git checkout -b|-B <name> <sha>` instead of
      `git checkout --detach <sha>` when `branch` is `Some`; `None` unchanged
- [ ] 1.4 Update `get_worktree(branch: Option<BranchMode>)` to thread the mode
      through to `create_slot` and `reset_slot`
- [ ] 1.5 Add unit / integration tests for the new `create_slot` and
      `reset_slot` branch paths — use `GitEnv` container helper

## 2. CLI — `-b` / `-B` arguments on `bs get`

- [ ] 2.1 Convert `Commands::Get` from a unit variant to a struct variant in
      `src/main.rs`, adding `new_branch: Option<String>` (`-b`,
      `conflicts_with = "reset_branch"`) and `reset_branch: Option<String>`
      (`-B`, `conflicts_with = "new_branch"`) fields
- [ ] 2.2 Map the parsed CLI flags to `Option<BranchMode>` and pass to
      `get_worktree`
- [ ] 2.3 Update the `match cli.command` arm so that `None` (default invocation)
      still calls `get_worktree(None)` unchanged
- [ ] 2.4 When a branch name is provided, append it to the stdout output line
      (e.g. `🌳 /path/to/slot  (my-feature)`)

## 3. Integration tests

- [ ] 3.1 E2E test: `bs get -b <branch>` creates the branch in the slot
      (container-isolated via `GitEnv`)
- [ ] 3.2 E2E test: `bs get -b <branch>` fails when the branch already exists
      (non-zero exit, stderr contains branch name)
- [ ] 3.3 E2E test: `bs get -B <branch>` creates the branch when it does not
      exist
- [ ] 3.4 E2E test: `bs get -B <branch>` resets an existing branch to HEAD
      without error
- [ ] 3.5 E2E test: `bs get` with no flags still produces a detached HEAD slot
      (regression guard)
- [ ] 3.6 Unit test: passing both `-b` and `-B` to the CLI exits non-zero (clap
      `conflicts_with` enforcement)

## 4. Documentation & cleanup

- [ ] 4.1 Update the `Get` subcommand doc comment in `src/main.rs` to describe
      the `-b`/`-B` flags and their semantics
- [ ] 4.2 Remove the `- [ ] -b and -B options on get subcommand` entry from
      `TODO.md`
- [ ] 4.3 Run `mise run test` and confirm all tests pass
