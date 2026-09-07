# Design: Quiet preserved slot in `bs prune`

## Approach

Remove the `kept ...` printing block in `src/main.rs`'s `Commands::Prune`
handler. Broaden the "nothing to prune" early-return condition so it also covers
the case where `outcome.preserved.is_some()` but `outcome.pruned` is empty and
there were no failures — i.e., treat a lone successful preserve the same as
"nothing to prune."

Preserve-failure reporting (`outcome.preserve_failure`) is untouched: it's a
real error and must still surface with a non-zero exit.

No changes to `src/worktree/mod.rs` — the selection/detach logic is correct and
only the CLI's reporting layer changes.

## Trade-offs

- Slight loss of transparency (a curious user can no longer see from `bs prune`
  output which slot is being kept warm; `bs list`/`bs status` still show it).
  Accepted per explicit user request: output should look exactly like "nothing
  to prune."
