## Context

`bs list` (`src/main.rs`, `Commands::List` handler) prints one line per pool
worktree slot in the form:

```
{prefix}{badge}  {path_display}
```

`badge` is one of `"locked"`, `"in use"`, or `"available"` (already colorized
via `owo-colors`, e.g. `"available".green().to_string()`). A fixed two-space
separator is hardcoded between badge and path, so rows with `"available"` (9
chars) push the path 3 columns further right than rows with
`"locked"`/`"in use"` (6 chars each), producing the misalignment reported by the
user.

This is a small, localized formatting fix confined to the print loop in
`Commands::List`; no other command or the underlying classification logic
(`worktree::list_worktrees_status`, `classify_slot_status`) is affected.

## Goals / Non-Goals

**Goals:**

- Make the worktree path column start at the same screen column on every row of
  `bs list` output, regardless of which status badge is shown.
- Keep the fix self-contained to the display/formatting layer.

**Non-Goals:**

- Changing badge wording, colors, or the set of possible statuses.
- Changing the `▶`/two-space current-slot prefix behavior.
- Introducing a general-purpose table-formatting abstraction/crate.
- Aligning any other `bs` subcommand's output (only `bs list` is affected by
  this bug).

## Decisions

- **Compute a fixed pad width from the plain (uncolored) badge strings, not
  colorized ones.** ANSI color escape codes are zero-width visually but count as
  characters in a plain string length calculation. Padding must be based on the
  _visible_ text length (`"locked"`, `"in use"`, `"available"` → max length 9)
  applied via a manual `format!("{:<width$}", ...)` on the plain label, with
  color applied to the whole padded string (or applied first and the invisible
  ANSI bytes accounted for) so the colorizer never distorts the padding math.
  - Alternative considered: pad after colorizing, using
    `format!("{:<width$}", colored_string)`. Rejected because `owo-colors`'
    colored wrapper still yields correct `Display` width in practice for simple
    color-only usage (no other styling), but relying on that is fragile across
    style combinations; computing width from the plain label first is more
    robust and easier to reason about.
- **Hardcode the pad width to the length of `"available"` (9)** rather than
  computing it dynamically from the actual set of statuses present in a given
  `bs list` invocation. A fixed constant keeps output stable across runs (a pool
  with only `locked`/`in use` slots still aligns the same way as one that also
  has `available` slots, and column width won't shift if the pool composition
  changes between invocations), and avoids a pre-pass over `entries` just to
  compute a width.
  - Alternative considered: compute `max_width` dynamically from the actual
    badges present each run. Rejected: adds complexity for no benefit, and
    produces inconsistent column widths across different invocations/pools,
    which is arguably a worse UX than a fixed width.
- **Keep the two-space gap after the badge**, now measured after padding (i.e.,
  `{prefix}{padded_badge}  {path_display}`) rather than immediately after the
  raw badge text, so there's still a visible gap on every row.

## Risks / Trade-offs

- [Risk: any external script parses `bs list` output assuming exactly the
  literal strings `"in use "` / `"locked "` / `"available "` with a fixed
  2-space run regardless of column alignment] → Mitigation: this is an
  unsupported/undocumented output format (no `--json` or machine-readable mode
  for `bs list` exists); the proposal calls out that inter-column spacing
  changes as expected. No stable machine-readable contract is broken.
- [Risk: hardcoding the max badge width (9, for `"available"`) means if a new
  status variant with a longer label is added later, alignment silently breaks
  again] → Mitigation: compute the constant from the three known badge literals
  in one place (e.g., a small helper or constant array) so future additions are
  a one-line change alongside adding the new status variant, and call this out
  in a code comment at the definition site.

## Migration Plan

Not applicable — this is a non-breaking, purely cosmetic stdout formatting fix
in a single command's print loop. No data migration, config change, or rollback
procedure is needed; reverting the commit fully reverts the behavior.
