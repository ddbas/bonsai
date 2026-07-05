## Context

`bs` is a Rust CLI binary (clap 4.x) for bonsai project tooling. It currently
exposes a single `help` subcommand and gates all invocations behind
`arg_required_else_help`, so running `bs` with no arguments prints help. We are
adding a `get` command that provisions managed git worktrees from a reusable
pool. Git worktrees share a single `.git` object store — the reuse value comes
from keeping idle worktrees alive so cached build artifacts survive across
invocations. `bs get` can be called from the main worktree or from any linked
worktree (e.g. from inside `~/.bonsai/bonsai/some-slot`).

## Goals / Non-Goals

**Goals:**

- Introduce a `get` subcommand; make it the default (`bs` alone = `bs get`).
- Determine HEAD from the calling worktree's current commit.
- Resolve the canonical main repo root correctly whether called from the main
  worktree or a linked worktree.
- Define a pool layout under `~/.bonsai/<repo-slug>/<uuid-slot>/`.
- Scan the pool for an available (clean + unlocked) slot; reset it to HEAD
  (detached) if found.
- Create a new UUID-named slot if the pool has no available worktree.
- Print the provisioned worktree path to stdout.

**Non-Goals:**

- Branch creation, checkout, or any branch management inside the worktree
  (user's responsibility).
- Running post-checkout setup scripts (e.g. `npm install`).
- Remote/push operations.
- Cross-repo worktree management.
- Explicit in-use locking beyond what git already tracks (dirty state + git
  lock).

## Decisions

### 1 — Default command via clap

**Decision**: Change `command: Commands` to `command: Option<Commands>` and
remove `arg_required_else_help`. `None` dispatches to `Get` with defaults.

Clap 4.x has no built-in "default subcommand" mechanism. `Option<Commands>` is
the standard idiom:

```rust
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}
// main: match cli.command { None => run_get(...), Some(cmd) => ... }
```

**Alternatives considered**: `flatten` + custom `FromArgMatches` — more complex,
no benefit here.

### 2 — Worktree pool layout

**Decision**:

```
~/.bonsai/
  <repo-slug>/          ← derived from main repo root (see Decision 3)
    <uuid>/             ← 8-char UUID v4 prefix, e.g. "a3f9c1b2"
    <uuid>/
```

Slot names are always UUID-derived, never branch-derived. Branch names are
transient and become misleading once a slot is reused for a different purpose.
UUIDs are stable, opaque identifiers for pool entries.

**Alternatives considered**:

- Branch-derived names (e.g. `feature-my-task`): misleading after reuse.
- Sequential integers: collision-prone across processes; harder to reason about.

### 3 — Repo root resolution (works from any worktree)

**Decision**: Use `git rev-parse --git-common-dir` to locate the shared `.git`
directory, then take its parent as the main repo root. This is correct whether
called from the main worktree or any linked worktree.

`git rev-parse --show-toplevel` returns the _calling_ worktree's directory (e.g.
`~/.bonsai/bonsai/a3f9c1b2`), not the main repo root — making slug derivation
wrong when called from a managed worktree.

`--git-common-dir` always returns the shared git dir (e.g.
`/Users/david/repos/bonsai/.git`). Its parent is the canonical repo root.

```
repo-slug = basename(parent(git_common_dir)).to_lowercase(), non-alphanumeric → '-'
```

**Alternatives considered**: Parse `git worktree list --porcelain` and take the
first entry's path — works, but `--git-common-dir` is more direct and doesn't
require parsing.

### 4 — HEAD determination

**Decision**: Run `git rev-parse HEAD` in the current working directory (i.e.
the calling worktree). This gives the commit SHA of whichever worktree `bs get`
was invoked from.

All created/reset worktrees are **detached HEAD** at this SHA. Branch creation
is explicitly out of scope.

### 5 — Availability check

**Decision**: A pool slot is **available** if:

1. Its path exists on disk.
2. It is **not locked** — no `locked` line in `git worktree list --porcelain`
   for that entry.
3. Its working tree is **clean** — `git -C <slot-path> status --porcelain`
   returns empty output.

Dirty working tree = uncommitted changes = slot is in active use. Locked =
explicitly held. Either condition marks the slot as unavailable.

Stale slots (registered but directory missing) are pruned with
`git worktree prune` before the scan.

### 6 — Reset vs create

**If an available slot is found:**

```
git -C <slot-path> checkout --detach <HEAD-SHA>
```

This switches the slot to detached HEAD at the target commit regardless of its
current branch/commit state. Safe because we've verified the working tree is
clean.

**If no available slot exists:**

```
git worktree add --detach <slot-path> <HEAD-SHA>
```

`slot-path` = `<root>/<repo-slug>/<8-char-uuid>/`

### 7 — Argument surface

```
bs get
```

No arguments, no options. The pool location (`~/.bonsai/<repo-slug>/`) is fixed.
The pool is opaque — callers receive a path and use it.

### 8 — Dependencies

- `dirs = "5"`: cross-platform `home_dir()`. Falls back to `$HOME` env var if
  `dirs::home_dir()` returns `None`.
- `uuid = { version = "1", features = ["v4"] }`: UUID v4 generation for slot
  names.

## Risks / Trade-offs

- **`checkout --detach` fails if working tree has conflicts** → Mitigated by the
  clean-check (step 5); a conflicted tree will show in `status --porcelain`.
- **Concurrent `bs get` calls race on the same available slot** → Two processes
  could both see the same slot as available and both try to reset it.
  Mitigation: acceptable for now (last writer wins on the `checkout --detach`);
  a future change can use `git worktree lock`/`unlock` as a mutex.
- **Cross-platform home dir**: `dirs::home_dir()` returns `None` in some CI
  sandboxes. Mitigation: clear error message directing the user to ensure
  `$HOME` is set.
- **repo-slug collisions**: Two repos with the same directory basename share a
  pool namespace. Mitigation: acceptable for now; future change can append a
  short hash of the full path.
- **`--git-common-dir` in bare repos**: Returns `.` for bare repos. Mitigation:
  detect and error out with a clear message — bare repos are not a supported use
  case.
