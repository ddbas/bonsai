# Rust tests not ported as e2e (checklist for task group 4)

All `tests/*.rs` files are ported to `apps/bonsai/tests/e2e/*.test.ts` (including `classification_agreement.rs` and `e2e_example.rs`; `common/mod.rs` → `support/index.ts`). `benches/bs_ls.rs` → task 7.1.

Inline `#[cfg(test)]` tests to port as unit tests in the owning TS package (tick when ported):

## src/logging.rs
- [x] `test_log_dir_returns_some_path`
- [x] `test_log_level_display`
- [x] `test_prune_logs_keeps_recent_files`
- [x] `test_prune_logs_ignores_non_matching_files`
- [x] `test_init_with_nonwritable_directory_returns_none`
- [x] `test_current_log_file_returns_path`
- [x] `test_current_log_file_has_date_suffix`

## src/main.rs
- [x] `visible_width_no_branch`
- [x] `visible_width_with_branch`
- [x] `padding_aligns_stats_column`
- [x] `current_slot_path_display_no_branch`
- [x] `current_slot_path_display_with_branch`
- [x] `non_current_slot_path_display_has_no_current_label`
- [x] `current_and_non_current_same_visible_width`
- [x] `current_slot_column_alignment_preserved`
- [x] `format_current_path_no_branch`
- [x] `format_current_path_with_branch`

## src/tmux.rs
- [x] `default_session_name_matches_worktree_get_convention`
- [x] `default_session_name_uses_detached_label`
- [x] `resolve_session_name_empty_value_derives_default`
- [x] `resolve_session_name_non_empty_value_used_verbatim`

## src/worktree/branch_state.rs
- [x] `availability`
- [x] `unpushed_reason_mentions_commits_upstream_and_hint`
- [x] `singular_commit`
- [x] `other_reasons`
- [x] `resolve_falls_back_to_local_main_without_remote`
- [x] `resolve_none_when_origin_exists_without_refs`
- [x] `resolve_prefers_origin_main_then_master_and_origin_head`
- [x] `branch_state_ancestry_unpushed_and_default_unknown`

## src/worktree/mod.rs
- [x] `tilde_path_replaces_home_prefix`
- [x] `tilde_path_exact_home`
- [x] `tilde_path_outside_home_unchanged`
- [x] `classify_locked_beats_everything`
- [x] `classify_dirty_unlocked_is_in_use`
- [x] `classify_unmerged_work_unlocked_is_in_use`
- [x] `classify_dirty_and_unmerged_work_is_in_use`
- [x] `classify_clean_unlocked_no_unmerged_work_is_available`
- [x] `classify_slot_status_locked_short_circuits`
- [x] `classify_slot_status_unlocked_dirty_is_in_use`
- [x] `classify_slot_status_and_slot_status_agree_via_classify`
- [x] `parse_branch_refs_heads_strips_prefix`
- [x] `parse_branch_refs_heads_nested`
- [x] `parse_branch_detached_yields_none`
- [x] `parse_locked_line_no_lock`
- [x] `parse_locked_line_locked_no_reason`
- [x] `parse_locked_line_locked_with_reason`
- [x] `git_status_lines_classifies_modified_and_untracked`
- [x] `git_status_lines_empty_output`
- [x] `branch_mode_new_holds_name`
- [x] `branch_mode_reset_holds_name`
- [x] `branch_mode_new_and_reset_are_distinct`
- [x] `branch_mode_clone`
- [x] `resolve_head_and_common_dir_returns_both_values`
- [x] `resolve_head_and_common_dir_matches_individual_calls`
- [x] `list_pool_worktrees_checking_stale_no_stale_for_fresh_dir`
- [x] `slugify_lowercases_ascii`
- [x] `slugify_replaces_dot_with_dash`
- [x] `slugify_replaces_space_with_dash`
- [x] `slugify_replaces_multiple_non_alnum`
- [x] `slugify_preserves_digits`
- [x] `new_slot_path_has_eight_char_hex_name`
- [x] `new_slot_path_parent_is_pool_dir`
- [x] `new_slot_path_successive_calls_differ`
- [x] `validate_pool_slot_accepts_valid_path`
- [x] `validate_pool_slot_rejects_path_outside_pool`
- [x] `validate_pool_slot_rejects_nonexistent_path`
- [x] `find_slot_for_cwd_returns_none_for_absent_pool`
- [x] `find_slot_for_cwd_matches_slot_ancestor`
- [x] `find_slot_checked_out_on_branch_returns_none_for_absent_pool`
- [x] `find_slot_checked_out_on_branch_returns_none_when_no_match`
- [x] `find_slot_checked_out_on_branch_matches_single_slot`
- [x] `find_slot_checked_out_on_branch_matches_locked_slot`
- [x] `configured_copy_paths_unset_returns_empty`
- [x] `configured_copy_paths_single_entry`
- [x] `configured_copy_paths_multiple_entries_preserve_order`
- [x] `copy_ignored_files_copies_existing_file`
- [x] `copy_ignored_files_skips_missing_source_and_continues`
- [x] `copy_ignored_files_creates_destination_subdirectory`
- [x] `copy_ignored_files_empty_list_is_noop`
- [x] `select_prune_candidates_empty_input_preserves_nothing`
- [x] `select_prune_candidates_single_entry_is_preserved_not_deleted`
- [x] `select_prune_candidates_multiple_entries_preserves_first_only`


## Port notes

- `branch_mode_*` (type-only tests): the TypeScript `BranchMode` is a
  discriminated union checked by `tsc`; behaviour is covered by the get/git
  command-construction tests.
- `new_slot_path_*` → `apps/bonsai/src/runtime.test.ts` (slot id) and
  `packages/core/tests/pool.test.ts` (pool placement).
- `parse_*`, `git_status_lines_*`, `configured_copy_paths_*`,
  `resolve_head_and_common_dir_*` → `packages/git/tests/`.
- `classify_*`, `slugify_*`, `tilde_path_*`, `validate_pool_slot_*`,
  `find_slot_*`, `copy_ignored_files_*`, `select_prune_candidates_*` →
  `packages/core/tests/`.
- `logging.rs` tests → `packages/logging/tests/logging.test.ts`;
  `tmux.rs` → `packages/core/tests/tmux.test.ts` + `packages/tmux/tests/`;
  `main.rs` formatting tests → `apps/bonsai/src/program.test.ts`.
