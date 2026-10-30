## MODIFIED Requirements

### Requirement: `bs list` meets a documented performance SLO, calibrated per slot-state scenario

`bs list`'s per-slot classification latency SHALL be tracked and enforced by an
automated performance test (run as part of `mise run test`), measured as p95
latency over repeated in-process invocations of the classification path on a
warm filesystem cache, with SLOs calibrated separately per slot-state scenario
since the achievable bound depends on how many slots require a
`git status`/branch-state call:

1. **All slots locked** (no subprocess fan-out): at a pool size of 50 managed
   worktree slots, p95 latency SHALL be **<= 100ms**, and the scaling ratio of
   p95 at 50 slots vs. p95 at 5 slots SHALL be **<= 6.0x**.
2. **All slots dirty/unlocked** (one `git status` per slot): at a pool size of
   50 slots, p95 latency SHALL be **<= 100ms**. Because cost grows near-linearly
   with pool size, the scaling ratio of p95 at 50 slots vs. p95 at 5 slots SHALL
   be checked only against the generous scaling-regression bound of **<= 15x**,
   not 6.0x.
3. **All slots clean, unlocked, and available** (every slot requires both a
   `git status` and a branch-state check — the classification cost floor):
   latency is tracked and reported by the test but is **not** subject to a fixed
   absolute bound; instead, it is checked against the same scaling-regression
   bound (**<= 15x**), to catch a regression beyond the expected cost of one
   `git status` + one branch-state check per slot, without demanding sub-linear
   scaling that isn't achievable when every slot must be checked.

This test and its thresholds exist specifically to catch (a) a regression back
to per-slot subprocess fan-out for scenarios where early return should have
avoided it (scenario 1), and (b) an unexpected additional cost beyond the
inherent `git status`/branch-state floor for scenarios 2 and 3.

#### Scenario: CI fails on an SLO violation in the locked/dirty scenarios

- **WHEN** the performance test runs (locally via `mise run test`, in the
  `pre-commit` git hook, or in CI on every push/PR) against the all-locked or
  all-dirty scenarios
- **THEN** it SHALL fail and print which SLO was violated (absolute p95 bound,
  scaling ratio bound, or both) if that scenario's thresholds above are exceeded
- **THEN** it SHALL pass and print the measured p95 values when all thresholds
  are met

#### Scenario: All-available scenario is reported but not hard-gated on an absolute bound

- **WHEN** the performance test runs against the all-available performance-test
  scenario
- **THEN** it SHALL print the measured p95 values at each pool size
- **THEN** it SHALL fail only if the scaling ratio exceeds the 15x regression
  bound, not merely for being slower than the locked/dirty scenarios' absolute
  bound
