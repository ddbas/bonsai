## Why

`bs prune` now silently preserves one `available` slot instead of deleting it,
which is correct behavior. But it also prints a distinct `kept <path>` (or
`kept <path>  (detached branch)`) line for that preserved slot. This extra line
is noise: the preserved slot was never a problem the user needed to know about,
and the "kept" wording implies an action was taken on their behalf that they now
have to parse. The preserved slot should be an invisible, ordinary part of the
pool — the CLI output should read exactly as it would if that slot had never
been a candidate for pruning at all.

## What Changes

- `bs prune` SHALL NOT print any line reporting the preserved slot (no
  `kept ...` output), whether or not it had a branch detached.
- When the preserved slot is the only slot that would have been touched (no
  other slots were pruned and no failures occurred), `bs prune` SHALL print the
  same "nothing to prune" message it prints when there were zero available
  slots, so the output is indistinguishable from a run where nothing happened.
- The preserve-and-detach behavior itself (selecting one available slot and
  detaching its branch) is unchanged; only the reporting of it is removed.
- Detach _failures_ for the preserved slot SHALL continue to be reported as an
  error and continue to cause a non-zero exit status, since that is an actual
  failure the user needs to know about.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `worktree-prune`: removes the requirement that the preserved slot be reported
  distinctly from deleted slots; clarifies that a successful preserve produces
  no additional output and, when nothing else happened, the run reports "nothing
  to prune."

## Impact

- `src/main.rs`: remove the `kept ...` printing for `outcome.preserved`, and
  broaden the "nothing to prune" condition to include the case where only a
  successful preserve occurred (no pruned slots, no failures, no preserve
  failure).
- `openspec/specs/worktree-prune/spec.md`: drop the "Preserved slot is reported
  distinctly from deleted slots" requirement and adjust related scenarios.
