## Context

`bs list` currently emits three pieces of information per slot: a colour-coded
status badge (`available` / `in use`), the tilde-abbreviated slot path, and an
optional raw process count. Two gaps make the output insufficient for a quick
"is this safe to reclaim?" decision:

1. **No branch context**: users cannot see which branch (if any) is checked out
   without switching into the worktree and running `git branch`.
2. **Coarse in-use reason**: a single process count does not distinguish between
   open-editor processes, uncommitted modifications, or untracked build
   artifacts — all of which prevent safe reuse in different ways.

The `git worktree list --porcelain` output already contains a `branch` field, so
branch data is free. Git already provides dirty/untracked counts via
`git status --porcelain`. No new binary dependencies are introduced.

## Goals / Non-Goals

**Goals:**

- Append the current branch name (bold, parenthesised) to the path column.
- Replace the raw process count with a compact, icon-driven stats field that
  shows only non-zero values for: open processes (`⚙`), uncommitted files (`±`),
  untracked files (`?`).
- Keep the change purely additive in `lib.rs`; the public `WorktreeEntry` type
  gains a `branch` field and `list_worktrees_status` gains extra stat fields.
- Keep line width reasonable: the stats column is empty for clean slots and at
  most `⚙99 ±99 ?99` (~14 chars) for busy ones.

**Non-Goals:**

- Machine-readable (JSON) output mode for `bs list`.
- Sorting or filtering by branch or stat.
- Showing staged vs. unstaged separately (both are rolled into `±`).
- Any change to `bs get` behaviour.

## Decisions

### D1 — Branch from `git worktree list --porcelain`, not `git -C <slot> branch`

`git worktree list --porcelain` already emits a `branch refs/heads/<name>` line
for attached worktrees and a bare `detached` line for detached HEAD. Parsing
this is cheap (no extra subprocess) and consistent with how
`list_pool_worktrees` already works.

Alternatives considered:

- `git -C <slot> rev-parse --abbrev-ref HEAD` — one extra subprocess per slot;
  unnecessary when the data is already in porcelain output.

### D2 — Icon set: `⚙` / `±` / `?`

| Icon | Meaning                                 | Rationale                                                            |
| ---- | --------------------------------------- | -------------------------------------------------------------------- |
| `⚙`  | Open processes                          | Gear = active/running; universally understood                        |
| `±`  | Uncommitted changes (modified + staged) | Plus-minus = "something changed"; already used in git prompt tooling |
| `?`  | Untracked files                         | Standard git short-status prefix; immediately recognisable           |

Only non-zero stats are rendered; an available (clean + idle) slot displays an
empty stats field.

Alternatives considered:

- Text abbreviations (`P`, `M`, `U`) — readable but wider and less distinctive
  at a glance.
- Emoji (🔄, 📝, ❓) — inconsistent terminal width across font stacks; rejected
  for reliability.

### D3 — Uncommitted count from `git status --porcelain`, untracked separately

`git status --porcelain` outputs one line per file with a two-character XY
status code. Lines where either X or Y is not `?` count as modified/staged
(`±`). Lines where both characters are `?` count as untracked (`?`). This is a
single subprocess and gives both counts in one pass.

### D4 — `WorktreeEntry` gains `branch: Option<String>` field; `list_worktrees_status` returns `WorktreeStats`

Introducing a small `WorktreeStats` struct avoids a growing positional tuple:

```rust
pub struct WorktreeStats {
    pub process_count: usize,
    pub uncommitted_count: usize,
    pub untracked_count: usize,
}
```

`list_worktrees_status` changes return type from
`Vec<(PathBuf, WorktreeStatus, Option<usize>)>` to
`Vec<(PathBuf, WorktreeStatus, WorktreeStats, Option<String>)>` (path, status,
stats, branch). The `branch` field is `None` for detached HEAD.

This is a breaking change to the public API within the same crate; all callers
are in `main.rs` and are updated together.

## Risks / Trade-offs

- **Terminal font support** → `⚙` and `±` are BMP Unicode and render correctly
  in all modern terminals; `?` is ASCII. Risk is low.
- **`git status --porcelain` cost** → One extra subprocess per slot. For a
  typical pool of 2–5 slots this is negligible. For very large pools (10+) it
  could add ~50 ms. Mitigation: acceptable for a human-facing command; no
  caching needed at this scale.
- **Breaking internal API** → `list_worktrees_status` signature changes. The
  function is `pub` but used only within this binary; no downstream crate
  consumers exist. All call sites are updated atomically.
