## 1. Implementation

- [x] 1.1 In `src/main.rs`'s `Commands::List` handler, define a constant (or
      small helper) for the fixed badge column width, derived from the length of
      the longest plain badge label (`"available"` = 9 chars), with a comment
      noting it must be updated if a new status variant with a longer label is
      added.
- [x] 1.2 Compute each row's plain badge label (`"locked"` / `"in use"` /
      `"available"`) separately from its colorized form, and left-pad the plain
      label to the fixed width using `format!("{:<width$}", ...)` before
      applying color, so the padding calculation is never distorted by ANSI
      escape codes.
- [x] 1.3 Update the row `println!` in `Commands::List` to print
      `{prefix}{padded_colored_badge}  {path_display}` so the two-space gap is
      applied after padding, keeping the worktree path column fixed across all
      rows.

## 2. Tests

- [x] 2.1 Add a test in `tests/worktree_list.rs` covering a mixed pool with at
      least one `available` slot and one `in use`/`locked` slot, asserting that
      the worktree path substring starts at the same character column (byte
      offset from line start, ignoring ANSI codes) on every line of `bs list`
      output.
- [x] 2.2 Add/extend a test covering alignment when the `▶` current-slot prefix
      is present alongside slots of differing badge lengths, asserting the path
      column position is unaffected by the prefix.
- [x] 2.3 Run `cargo test` and confirm all existing `worktree_list` tests
      (single slot, branch display, dirty slot, locked slot, `ls` alias, empty
      pool) still pass unchanged.

## 3. Validation

- [x] 3.1 Manually run `bs list` in a pool with slots in `locked`, `in use`, and
      `available` states and visually confirm the path column is aligned.
- [x] 3.2 Run `openspec validate fix-list-column-alignment --strict` (or
      equivalent) to confirm the change's specs are well-formed before
      archiving.
