## 1. Data Model & Library Changes

- [x] 1.1 Add `branch: Option<String>` field to `WorktreeEntry` struct in
      `src/worktree/mod.rs`
- [x] 1.2 Update `list_pool_worktrees` to parse the `branch refs/heads/<name>`
      (and `detached`) line from `git worktree list --porcelain` output and
      populate the new `branch` field
- [x] 1.3 Add `WorktreeStats` struct with `process_count: usize`,
      `uncommitted_count: usize`, `untracked_count: usize` fields
- [x] 1.4 Add
      `count_git_status_files(slot_path: &Path) -> Result<(usize, usize)>`
      helper that runs `git -C <slot> status --porcelain` and returns
      `(uncommitted_count, untracked_count)` by classifying each output line by
      its XY code
- [x] 1.5 Update `list_worktrees_status` return type from
      `Vec<(PathBuf, WorktreeStatus, Option<usize>)>` to
      `Vec<(PathBuf, WorktreeStatus, WorktreeStats, Option<String>)>` and
      populate all fields

## 2. Unit Tests for New Logic

- [x] 2.1 Add unit test for `list_pool_worktrees` branch parsing:
      `branch refs/heads/main` → `Some("main".to_string())`, detached → `None`
- [x] 2.2 Add unit test for `count_git_status_files` parsing: verify `??` lines
      count as untracked, non-`??` lines count as uncommitted, mixed input
- [x] 2.3 Add unit test for the stats column formatter (see task 3.1): all-zero
      → empty string, partial non-zero → correct icon+number output, all three
      non-zero → `⚙1 ±2 ?3`
- [x] 2.4 Update existing `synthetic_status` tests in `mod.rs` to pass
      `WorktreeStats` instead of bare counts if necessary

## 3. Rendering Changes in `src/main.rs`

- [x] 3.1 Add a `format_stats(stats: &WorktreeStats) -> String` helper that
      builds the compact stats string (`⚙N ±N ?N`, omitting zero components)
- [x] 3.2 Update the `List` command rendering loop to:
  - Append the branch name in bold parentheses after the tilde path (when
    `branch` is `Some`)
  - Replace the raw process-count column with the output of `format_stats`

## 4. Integration Test

- [x] 4.1 Add a `GitEnv`-based integration test that provisions a worktree,
      checks out a branch, adds an untracked file, and asserts `bs list` output
      includes the bold branch name and `?1` in the stats column
