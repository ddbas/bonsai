# Rust tests not ported as e2e (checklist for task group 4)

All `tests/*.rs` files are ported to `apps/bonsai/tests/e2e/*.test.ts` (including `classification_agreement.rs` and `e2e_example.rs`; `common/mod.rs` → `support/index.ts`). `benches/bs_ls.rs` → task 7.1.

Inline `#[cfg(test)]` tests to port as unit tests in the owning TS package (tick when ported):

## src/logging.rs
- [ ] `test_log_dir_returns_some_path`
- [ ] `test_log_level_display`
- [ ] `test_prune_logs_keeps_recent_files`
- [ ] `test_prune_logs_ignores_non_matching_files`
- [ ] `test_init_with_nonwritable_directory_returns_none`
- [ ] `test_current_log_file_returns_path`
- [ ] `test_current_log_file_has_date_suffix`

## src/main.rs
- [ ] `visible_width_no_branch`
- [ ] `visible_width_with_branch`
- [ ] `padding_aligns_stats_column`
- [ ] `current_slot_path_display_no_branch`
- [ ] `current_slot_path_display_with_branch`
- [ ] `non_current_slot_path_display_has_no_current_label`
- [ ] `current_and_non_current_same_visible_width`
- [ ] `current_slot_column_alignment_preserved`
- [ ] `format_current_path_no_branch`
- [ ] `format_current_path_with_branch`

## src/tmux.rs
- [ ] `default_session_name_matches_worktree_get_convention`
- [ ] `default_session_name_uses_detached_label`
- [ ] `resolve_session_name_empty_value_derives_default`
- [ ] `resolve_session_name_non_empty_value_used_verbatim`

## src/worktree/branch_state.rs
- [ ] `availability`
- [ ] `unpushed_reason_mentions_commits_upstream_and_hint`
- [ ] `singular_commit`
- [ ] `other_reasons`
- [ ] `resolve_falls_back_to_local_main_without_remote`
- [ ] `resolve_none_when_origin_exists_without_refs`
- [ ] `resolve_prefers_origin_main_then_master_and_origin_head`
- [ ] `branch_state_ancestry_unpushed_and_default_unknown`

## src/worktree/mod.rs
- [ ] `tilde_path_replaces_home_prefix`
- [ ] `tilde_path_exact_home`
- [ ] `tilde_path_outside_home_unchanged`
- [ ] `classify_locked_beats_everything`
- [ ] `classify_dirty_unlocked_is_in_use`
- [ ] `classify_unmerged_work_unlocked_is_in_use`
- [ ] `classify_dirty_and_unmerged_work_is_in_use`
- [ ] `classify_clean_unlocked_no_unmerged_work_is_available`
- [ ] `classify_slot_status_locked_short_circuits`
- [ ] `classify_slot_status_unlocked_dirty_is_in_use`
- [ ] `classify_slot_status_and_slot_status_agree_via_classify`
- [ ] `parse_branch_refs_heads_strips_prefix`
- [ ] `parse_branch_refs_heads_nested`
- [ ] `parse_branch_detached_yields_none`
- [ ] `parse_locked_line_no_lock`
- [ ] `parse_locked_line_locked_no_reason`
- [ ] `parse_locked_line_locked_with_reason`
- [ ] `git_status_lines_classifies_modified_and_untracked`
- [ ] `git_status_lines_empty_output`
- [ ] `branch_mode_new_holds_name`
- [ ] `branch_mode_reset_holds_name`
- [ ] `branch_mode_new_and_reset_are_distinct`
- [ ] `branch_mode_clone`
- [ ] `resolve_head_and_common_dir_returns_both_values`
- [ ] `resolve_head_and_common_dir_matches_individual_calls`
- [ ] `list_pool_worktrees_checking_stale_no_stale_for_fresh_dir`
- [ ] `slugify_lowercases_ascii`
- [ ] `slugify_replaces_dot_with_dash`
- [ ] `slugify_replaces_space_with_dash`
- [ ] `slugify_replaces_multiple_non_alnum`
- [ ] `slugify_preserves_digits`
- [ ] `new_slot_path_has_eight_char_hex_name`
- [ ] `new_slot_path_parent_is_pool_dir`
- [ ] `new_slot_path_successive_calls_differ`
- [ ] `validate_pool_slot_accepts_valid_path`
- [ ] `validate_pool_slot_rejects_path_outside_pool`
- [ ] `validate_pool_slot_rejects_nonexistent_path`
- [ ] `find_slot_for_cwd_returns_none_for_absent_pool`
- [ ] `find_slot_for_cwd_matches_slot_ancestor`
- [ ] `find_slot_checked_out_on_branch_returns_none_for_absent_pool`
- [ ] `find_slot_checked_out_on_branch_returns_none_when_no_match`
- [ ] `find_slot_checked_out_on_branch_matches_single_slot`
- [ ] `find_slot_checked_out_on_branch_matches_locked_slot`
- [ ] `configured_copy_paths_unset_returns_empty`
- [ ] `configured_copy_paths_single_entry`
- [ ] `configured_copy_paths_multiple_entries_preserve_order`
- [ ] `copy_ignored_files_copies_existing_file`
- [ ] `copy_ignored_files_skips_missing_source_and_continues`
- [ ] `copy_ignored_files_creates_destination_subdirectory`
- [ ] `copy_ignored_files_empty_list_is_noop`
- [ ] `select_prune_candidates_empty_input_preserves_nothing`
- [ ] `select_prune_candidates_single_entry_is_preserved_not_deleted`
- [ ] `select_prune_candidates_multiple_entries_preserves_first_only`

