## 1. Spec Update

- [x] 1.1 Archive the delta spec into `openspec/specs/worktree-list/spec.md` by
      merging the MODIFIED requirement (the apply/archive phase handles this,
      but verify the spec file reflects the new scenarios after archiving)

## 2. Core Implementation

- [x] 2.1 In `src/main.rs`, inside the `Commands::List` arm, call
      `worktree::current_worktree().ok().flatten()` before the render loop to
      obtain the optional current-slot `PathBuf`
- [x] 2.2 Pass the current-slot path into the row-building loop and add a
      boolean `is_current` field to the `Row` struct by comparing each entry's
      `path` against the current-slot path
- [x] 2.3 In the render loop, prefix matching rows with `▶` and append
      ` (current)` after the branch/path segment; adjust `visible_width` to
      account for the added characters so column alignment is preserved

## 3. Housekeeping

- [x] 3.1 Remove the `- [ ] \`current\` subcommand: show the current
      worktree`checkbox from`TODO.md`

## 4. Tests

- [x] 4.1 Add a unit test (or extend an existing integration test) covering the
      scenario where `bs list` is run from inside a managed slot — verify the
      output line contains `▶` and `(current)`
- [x] 4.2 Add a test for the case where CWD is not inside any managed slot —
      verify no row contains `▶` or `(current)`
- [x] 4.3 Add a test confirming that when `current_worktree()` returns `None`
      (e.g. pool absent), `bs list` renders normally without panicking or
      emitting stray indicators

## 5. Verification

- [x] 5.1 Run `cargo test` and confirm all tests pass
- [x] 5.2 Manually run `bs list` from inside a provisioned slot and confirm the
      `▶ … (current)` indicator appears on the correct row
