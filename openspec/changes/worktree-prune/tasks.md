## 1. Core worktree helper logic

- [ ] 1.1 Add a `prune_available_slots(pool_dir: &Path)` (or similarly named)
      helper in `src/worktree/mod.rs` that calls `list_worktrees_status` to get
      `(path, status, branch)` for every pool slot, filters to
      `WorktreeStatus::Available`, and returns that filtered list (path +
      branch) for the caller to act on and report — reusing existing
      classification, no new porcelain parsing.
- [ ] 1.2 Add a helper that deletes a single slot's directory via
      `std::fs::remove_dir_all`, returning a `Result` so per-slot failures can
      be captured without aborting the whole run.
- [ ] 1.3 Add a helper that runs `git worktree prune` (reusing the existing
      `git_cmd()` builder), erroring with context if the git invocation itself
      fails to spawn or exits non-zero.
- [ ] 1.4 Wire the three helpers together in a single `prune_pool` (or similarly
      named) function: enumerate available slots, attempt to delete each
      directory (collecting successes and per-slot failures), then always run
      `git worktree prune` once at the end regardless of per-slot outcomes, and
      return enough information for the CLI layer to report pruned slots and any
      failures.

## 2. CLI wiring

- [ ] 2.1 Add a `Prune` variant to the `Commands` enum in `src/main.rs` with doc
      comments describing its behavior (mirroring the style of `List`/`Status`).
- [ ] 2.2 Handle the empty/non-existent pool case identically in spirit to
      `Commands::List`'s handling (pool dir missing → friendly message, exit 0)
      before invoking the prune logic.
- [ ] 2.3 Handle the "pool exists but nothing available" case with a distinct
      friendly message, still running `git worktree prune` before exiting 0.
- [ ] 2.4 On success, print one line per pruned slot using the existing
      tilde-path + bold-branch-in-parentheses formatting (reuse
      `worktree::tilde_path` and the same formatting pattern as `bs list`).
- [ ] 2.5 On partial failure (one or more slot deletions failed), print the
      failures clearly, still print the lines for slots that succeeded, and exit
      with a non-zero status; on full success exit 0.

## 3. Tests

- [ ] 3.1 Add `tests/worktree_prune.rs` following the `GitEnv`-based conventions
      of `tests/worktree_list.rs`.
- [ ] 3.2 Test: no pool directory yet → friendly message, exit 0, no git
      worktree prune errors.
- [ ] 3.3 Test: pool with only locked/in-use slots → no directories deleted,
      friendly "nothing to prune" message, exit 0.
- [ ] 3.4 Test: pool with a mix of locked, in-use, and available slots → only
      the available slot's directory is removed from disk, and after `bs prune`
      runs, `git worktree list --porcelain` (or `list_pool_worktrees`) no longer
      reports it, while the locked/in-use slots remain fully intact (directory
      present, still registered).
- [ ] 3.5 Test: pruned slot with a checked-out branch is reported with the
      branch name in parentheses; pruned detached-HEAD slot is reported with no
      branch suffix.
- [ ] 3.6 Test (best-effort, platform permitting): a per-slot deletion failure
      is reported and does not prevent other available slots from being pruned
      or `git worktree prune` from running; exit status is non-zero.

## 4. Documentation & polish

- [ ] 4.1 Update `README.md` (and any command reference/help text) to document
      `bs prune`, consistent with how `list`/`status`/`lock` are already
      documented.
- [ ] 4.2 Run `cargo fmt`, `cargo clippy`, and the full test suite; fix any
      warnings introduced by the new code.
