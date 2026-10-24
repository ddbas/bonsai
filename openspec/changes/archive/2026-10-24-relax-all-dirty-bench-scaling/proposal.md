## Why

The `bs ls` performance SLO in the `worktree-list` spec describes the all-dirty
scenario as having "no or partial subprocess fan-out" and holds it to a 6.0x
scaling ratio. Since availability is classified from git branch state (no `lsof`
short-circuit), every dirty slot runs its own `git status`, so cost scales
near-linearly with pool size. CI failed with a 7.4x ratio (44.9ms absolute, well
under the 100ms bound). The script was already relaxed; the spec is stale.

## What Changes

- Update the performance SLO requirement so the all-dirty scenario keeps the
  100ms absolute p95 bound at 50 slots but is gated on the generous
  scaling-regression bound (15x) instead of 6.0x.
- Only all-locked remains subject to the 6.0x ratio.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `worktree-list`: performance SLO requirement calibration for the all-dirty
  scenario.

## Impact

- `openspec/specs/worktree-list/spec.md`
- `scripts/check-bs-ls-perf.sh` (already updated in commit 049aece)
