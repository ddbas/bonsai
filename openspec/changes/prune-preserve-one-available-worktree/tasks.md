## 1. Core selection & preservation logic (`src/worktree/mod.rs`)

- [ ] 1.1 Update `prune_available_slots` (or add a new helper) so it returns the
      full list of available slots in pool order, and separately identify the
      "preserved" slot as the first entry when the list is non-empty.
- [ ] 1.2 Add a helper (e.g. `select_prune_candidates`) that, given the
      available-slots list, splits it into
      `(preserved: Option<(PathBuf,     Option<String>)>, to_delete: Vec<(PathBuf, Option<String>)>)`,
      preserving the first entry and returning the rest for deletion. Cover
      empty, single-entry, and multi-entry inputs with unit tests.
- [ ] 1.3 Extend `PruneOutcome` with a `preserved: Option<PrunedSlot>` field
      (and a way to represent a detach failure on the preserved slot, e.g.
      `preserve_failure: Option<(PathBuf, String)>`), updating its `Default`
      derive/impl as needed.

## 2. Detach-in-place for the preserved slot

- [ ] 2.1 In `prune_pool`, after computing `preserved`/`to_delete`: if
      `preserved` has `Some(branch)` (i.e. a branch is checked out), call
      `resolve_head()` then `reset_slot(path, &head_sha, None)` to detach it;
      leave slots already in detached HEAD (`branch == None`) untouched.
- [ ] 2.2 On detach success, populate `outcome.preserved` with the
      `PrunedSlot { path, branch }` capturing the _pre-detach_ branch name (so
      reporting can say what was detached).
- [ ] 2.3 On detach failure, populate the new failure field with the path and
      error message, do NOT delete that slot's directory, and ensure the overall
      run still proceeds to process `to_delete` and call `git_worktree_prune()`.
- [ ] 2.4 Continue deleting every slot in `to_delete` exactly as today
      (`delete_slot_dir`, collecting successes into `outcome.pruned` and
      failures into `outcome.failures`).
- [ ] 2.5 Ensure `prune_pool` still calls `git_worktree_prune()` exactly once at
      the end regardless of preserve/delete outcomes, and still exits
      non-zero-worthy (via existing failure-count logic) when the preserve step
      failed, matching how per-slot deletion failures are surfaced.

## 3. CLI reporting (`src/main.rs`)

- [ ] 3.1 Update the `bs prune` output to print a line for the preserved slot
      distinct from the pruned-slots list — e.g. "kept `<tilde-path>` (detached
      `<branch>`)" when a branch was detached, or "kept `<tilde-path>`" when it
      was already detached.
- [ ] 3.2 Update the "nothing to prune" branch's condition to also account for
      `outcome.preserved`/`preserve_failure` so it doesn't misreport when the
      only available slot was preserved (no deletions) but should still show
      what was kept.
- [ ] 3.3 Surface a preserve-detach failure the same way deletion failures are
      surfaced today (`eprintln!` + inclusion in the non-zero-exit failure
      count).
- [ ] 3.4 Update the `bs prune` subcommand's doc comment/help text to mention
      that one available slot is always preserved (and detached if needed).

## 4. Spec/regression tests

- [ ] 4.1 Add/update unit tests in `src/worktree/mod.rs` for
      `select_prune_candidates` (or equivalent): zero, one, and multiple
      available slots.
- [ ] 4.2 Add/update integration or unit-level tests for `prune_pool` covering:
      multiple available slots (one preserved, rest deleted); a single available
      slot with a branch (preserved + detached, not deleted); a single available
      slot already detached (preserved, no git op); zero available slots
      (unchanged "nothing to prune" behavior); detach failure on the preserved
      slot (not deleted, failure surfaced, other deletions and
      `git worktree prune` still proceed).
- [ ] 4.3 Update any existing tests/fixtures that assumed `bs prune` deletes
      _all_ available slots to instead assert one remains.
- [ ] 4.4 Run the full test suite (`cargo test`) and lint/format checks used by
      this repo (see `mise.toml`/`lefthook.yml`) before committing.

## 5. Finalize

- [ ] 5.1 Re-read
      `openspec/changes/prune-preserve-one-available-worktree/specs/worktree-prune/spec.md`
      against the final implementation and adjust wording/scenarios if the
      implementation diverged.
- [ ] 5.2 Commit and push the change per repo conventions.
