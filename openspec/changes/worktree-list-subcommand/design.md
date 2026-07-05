## Context

`bs` currently exposes a single meaningful subcommand: `get`. It manages a pool
of git worktrees under `~/.bonsai/<repo-slug>/`. Users who want to see the state
of their pool must inspect the filesystem directly or parse `git worktree list`
output by hand. A `list` subcommand bridges this gap with a concise, coloured
view.

Relevant existing code:

- `src/worktree/mod.rs`: `list_pool_worktrees(pool_dir)`, `is_clean(slot)`,
  `managed_root()`, `repo_slug()` — all reusable as-is.
- `src/main.rs`: `Commands` enum (clap `Subcommand` derive). Adding `List` here
  wires the new subcommand in.
- `Cargo.toml`: `owo-colors = "4"` is already a dependency; no new crates
  needed.

## Goals / Non-Goals

**Goals:**

- Add `bs list` (alias `bs ls`) that enumerates all bonsai-managed worktrees for
  the current repo's pool.
- Display each entry as `<status-badge>  <tilde-path>` where the badge is
  coloured green (available) or red (in use).
- "Available" = slot directory exists, not locked, and working tree is clean.
- "In use" = slot is locked OR has uncommitted changes.
- Home directory prefix is rendered as `~` in displayed paths.

**Non-Goals:**

- Listing worktrees from _other_ repos' pools.
- Showing branch or commit information per slot.
- Machine-readable (`--json`) output in this change.
- Interactive selection or navigation.

## Decisions

### Reuse existing pool utilities

`list_pool_worktrees` + `is_clean` + `WorktreeEntry.locked` already encode the
exact availability logic needed by `find_available_slot`. A thin new public
function `list_worktrees_status(pool_dir)` can wrap them and return
`Vec<(PathBuf, WorktreeStatus)>`.

**Alternative considered**: inline the logic directly in the `main.rs` handler.
Rejected — keeps display concerns mixed with pool logic.

### Use `owo-colors` (already in `Cargo.toml`)

No new dependency is required. `"available".green()` / `"in use".red()` via
`owo-colors::OwoColorize` gives ANSI colour without adding weight.

**Alternative**: `colored` crate. Rejected — `owo-colors` is already present.

### Clap alias `ls` on the `List` variant

`#[command(alias = "ls")]` on the `List` arm gives `bs ls` for free without a
separate `Ls` variant.

### Path display helper `tilde_path(path) -> String`

Extract a small helper in `worktree/mod.rs` that replaces the home directory
prefix with `~`. Using `dirs::home_dir()` (already a dep) keeps this portable.

### Emit one line per slot, no table headers

A minimal list (one line per worktree) is readable without a table header for
typical pool sizes (1–10 slots). Headers can be added in a follow-up if needed.

## Risks / Trade-offs

- **Empty pool**: If the pool directory does not exist yet (first run),
  `list_pool_worktrees` will fail to canonicalize. The handler should detect
  this case and print a friendly "no worktrees in pool" message rather than an
  error. → Mitigation: check `pool_dir.exists()` before calling
  `list_pool_worktrees`.
- **Colour in non-TTY contexts**: `owo-colors` emits ANSI codes unconditionally
  unless configured. For now this is acceptable (same pattern used if any future
  coloured output is added); pipe-friendliness is a non-goal here.
- **`is_clean` spawns a git subprocess per slot**: For small pools (typical
  case) this is negligible. For large pools it could be slow. → Acceptable for
  the current scale; parallelisation is a future concern.
