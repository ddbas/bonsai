## Context

`bs` manages a pool of git worktrees under `~/.bonsai/<repo-slug>/`. The CLI
already offers `bs get` (provision/reuse a slot) and `bs list` (survey all
slots). Identifying **which slot the shell is currently inside** requires the
user to visually match their CWD against `bs list` output — tedious and not
scriptable.

The current state of `src/worktree/mod.rs` provides all the building blocks
needed: `managed_root()`, `repo_slug()`, and `list_pool_worktrees()`. These let
us compute the pool directory and compare it against the process CWD.

## Goals / Non-Goals

**Goals:**

- Print the tilde-abbreviated path of the managed slot that contains the CWD.
- Optionally show the checked-out branch name (same format as `bs list`).
- Exit non-zero when the CWD is not inside any managed slot, enabling use in
  shell prompts and scripts (`if bs current; then …`).
- Reuse existing library functions; add one small public function
  `current_worktree()` to `src/worktree/mod.rs`.

**Non-Goals:**

- Detecting slots from _other_ repos (only the current repo's pool is checked).
- Showing usage stats (process count, dirty files) — that belongs to `bs list`.
- Changing the output format of any existing subcommand.

## Decisions

### Decision: CWD-prefix matching against pool entries

**Chosen approach**: Call `list_pool_worktrees(&pool_dir)` (already public),
then check whether the process CWD starts with any entry's path using
`Path::starts_with`.

**Alternative considered**: Re-parse `git worktree list --porcelain` and compare
paths without filtering by pool directory.

**Rationale**: Reusing `list_pool_worktrees` keeps the detection consistent with
`bs list`. We only care about pool-managed slots, not the main worktree or
ad-hoc linked worktrees. CWD-prefix matching correctly handles the case where
the user is in a _subdirectory_ of the slot (e.g.
`~/.bonsai/repo/a3f9c1b2/src`).

---

### Decision: Non-zero exit code when not in a managed slot

**Chosen approach**: Return `Ok(None)` from `current_worktree()` and let
`main.rs` translate that to a short human-readable message +
`std::process::exit(1)`.

**Alternative considered**: Return an `Err` and let the global error handler
print it.

**Rationale**: "Not in a managed slot" is not an error in the traditional sense;
it is a predictable, valid outcome. Using `Ok(None)` keeps it semantically
distinct from unexpected failures (e.g. git not found) and lets callers
distinguish the two cases.

---

### Decision: Output format mirrors `bs list` row

Print `<tilde-path>` or `<tilde-path> (<branch>)` — exactly the same format used
in the `list` rendering loop. This consistency makes it easy to mentally relate
`bs current` output to `bs list` output.

## Risks / Trade-offs

- [Risk: CWD resolution on symlinked paths] macOS often resolves `/tmp` to
  `/private/tmp`. `list_pool_worktrees` already canonicalises pool entries; we
  canonicalise the CWD via `std::env::current_dir()` + `.canonicalize()` before
  comparing. → Mitigation: existing precedent in `get_worktree`.

- [Risk: Pool directory does not exist yet] If the user has never run `bs get`,
  the pool directory is absent. → Mitigation: treat a missing pool directory the
  same as "not in a managed slot" (return `Ok(None)` without error).
