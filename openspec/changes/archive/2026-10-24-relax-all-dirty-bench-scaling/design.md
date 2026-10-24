## Context

All-dirty slots each require a `git status` call, identical in shape to the
all-available scenario's fan-out. Ideal linear scaling from 5 to 50 slots is
10x, so a 6.0x bound is unachievable and flaky.

## Decisions

- Keep the absolute p95 <= 100ms at 50 slots for all-dirty (still meaningful,
  ~45ms measured).
- Gate all-dirty scaling on the 15x regression bound shared with all-available.
- All-locked (no subprocess fan-out) keeps 6.0x.

## Risks / Trade-offs

- A looser ratio could hide moderate regressions; the absolute bound still
  catches them.
