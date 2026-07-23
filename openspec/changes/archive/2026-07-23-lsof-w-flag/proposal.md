## Why

`bs list` (and other commands that scan worktree pool slots) increasingly hit
`lsof: WARNING: can't stat() hfs file system` and similar diagnostics on macOS.
The codebase currently tries to filter these out by pattern-matching known
warning strings in `lsof`'s stderr, but new warning variants keep appearing that
the filter doesn't recognize, causing them to be misreported as real errors.
`lsof` has a built-in `-w` flag that suppresses all warning messages at the
source, making the custom filtering logic unnecessary.

## What Changes

- Add the `-w` flag to every `lsof` invocation in the `worktree` module (`+d`
  becomes `-w +d`) so `lsof` never emits warning diagnostics in the first place.
- Remove the `lsof_real_errors` filtering function and its call sites in
  `run_lsof` and `run_lsof_count` — stderr is used directly since `lsof -w`
  guarantees it contains only genuine errors.
- Remove tests and doc comments describing the Linux Docker overlay2/nsfs
  warning-line filtering and the macOS "assuming ... from mount table"
  filtering, since that logic no longer exists.

## Capabilities

### Modified Capabilities

- `worktree-open-file-detection`: `has_open_files` and `count_open_processes`
  now invoke `lsof -w +d <path>` instead of `lsof +d <path>`, and stderr is no
  longer filtered for known cosmetic warning patterns before being treated as an
  error.

## Impact

- `src/worktree/mod.rs`: `run_lsof`, `run_lsof_count`, and removal of
  `lsof_real_errors`.
- Unit tests in `src/worktree/mod.rs` that exercise warning-line filtering are
  removed or simplified.
