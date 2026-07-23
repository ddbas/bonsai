## Context

`run_lsof` and `run_lsof_count` in `src/worktree/mod.rs` invoke `lsof +d <path>`
and distinguish real errors from cosmetic diagnostics by pattern-matching known
warning strings in stderr (`lsof_real_errors`). This was originally added to
strip Linux Docker overlay2/nsfs warnings and a macOS "assuming ... from mount
table" line. New warning variants (e.g.
`lsof: WARNING: can't stat() hfs file system`) keep surfacing that the
hand-rolled filter doesn't recognize, causing `bs list`/`bs get` to fail with
what looks like a real error but is purely cosmetic.

`lsof` itself has a `-w` flag that suppresses all such warning messages at the
source, so `lsof` never writes them to stderr in the first place.

## Goals / Non-Goals

**Goals:**

- Stop misclassifying `lsof` warning diagnostics as real errors, for any warning
  variant, without maintaining a pattern list.
- Simplify `run_lsof`/`run_lsof_count` by removing the custom filtering step.

**Non-Goals:**

- Changing how genuine `lsof` errors (missing binary, permission failures on the
  target path) are surfaced — those behaviors are unchanged.

## Decisions

- **Add `-w` to the `lsof` command line instead of expanding the string
  filter.** Maintaining a growing list of warning-line patterns is a
  whack-a-mole fix; `-w` addresses the problem at its source and works for
  warning variants we haven't seen yet. Args become `["-w", "+d", <path>]`.
- **Delete `lsof_real_errors` entirely** rather than keeping it as a defensive
  fallback. With `-w` in place, any stderr output `lsof` produces is a genuine
  error, so the extra filtering step has no remaining purpose and would only
  hide real problems if kept.
- **Use stderr content directly (not exit code)** to decide success vs. error,
  unchanged from today — `lsof` still exits non-zero on macOS even when it
  successfully found no matches.

## Risks / Trade-offs

- [`-w` also suppresses permission-related warnings lsof would otherwise print
  for filesystems it can't read] → Acceptable: those warnings were already being
  treated as cosmetic by the existing filter's intent, and any failure that
  matters for the target path itself is reported as a hard error (empty stdout +
  non-empty stderr), not a warning.
