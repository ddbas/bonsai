## Why

`bs list` prints the status badge (`locked`, `in use`, `available`) followed by
the worktree path on each line, but the badge text is not padded to a fixed
width before the path is printed. Because `available` (9 chars) is longer than
`in use`/`locked` (6 chars each), the path column starts at a different screen
column depending on which badge a given row has, producing visually misaligned
output — e.g. paths in a `in use` row start two columns to the left of the path
in an `available` row. This makes the list harder to scan at a glance.

## What Changes

- Pad the status badge to the width of the longest possible badge string
  (`available`, 9 characters) before printing the worktree path, so the path
  column starts at the same screen column on every row regardless of which badge
  is shown.
- Padding is applied to the visible (uncolored) badge text; ANSI color codes
  applied via `owo-colors` MUST NOT affect the padding width calculation.
- No change to badge wording, colors, ordering, or the `▶`/two-space current
  slot prefix — only column alignment of the path after the badge.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `worktree-list`: The existing requirement "Each worktree is shown on its own
  line with path, branch, and status badge" is refined to require that the
  status badge is padded to a fixed column width so the worktree path column
  aligns across all rows regardless of badge text length.

## Impact

- Affected code: `src/main.rs` (`Commands::List` handler, where the badge and
  path are formatted and printed).
- No changes to `src/worktree/mod.rs` classification logic, CLI flags, or output
  content — purely a formatting/alignment fix to `bs list`'s stdout.
- No breaking changes to scripts parsing `bs list` output by badge word alone,
  though any script relying on a fixed number of spaces between badge and path
  will see that spacing change (previously 2 spaces always; now padding varies
  per badge to keep the path column fixed).
