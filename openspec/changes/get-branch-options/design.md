## Context

`bs get` today always provisions a worktree in detached HEAD state. The pool
management logic lives in `src/worktree/mod.rs` (`get_worktree`, `create_slot`,
`reset_slot`) and the CLI glue is in `src/main.rs` where `Commands::Get` is a
unit variant with no fields.

To follow a branch-based workflow a developer must currently run `bs get`, `cd`
into the slot, and then `git checkout -b <branch>` — three commands instead of
one.

## Goals / Non-Goals

**Goals:**

- Accept `-b <branch>` and `-B <branch>` on `bs get` at the CLI level.
- For **new** slots: use `git worktree add -b|-B <branch> <slot_path> <sha>` so
  the branch is set up in a single subprocess, with no separate checkout step.
- For **reused** slots: replace `git checkout --detach <sha>` with
  `git checkout -b|-B <branch> <sha>` directly, again avoiding a separate step.
- Print the branch name in the output line alongside the slot path.
- Keep all existing behaviour unchanged when neither flag is given.

**Non-Goals:**

- Tracking branch–slot affinity across invocations (a slot stays UUID-named; the
  branch is just checked out inside it).
- Changing pool layout, managed root location, or slot naming.
- Supporting branch creation for the `list` or any other subcommand.
- Implementing the `lock`, `config`, `current`, or `prune` subcommands mentioned
  in TODO.md (separate changes).

## Decisions

### Decision: Introduce a `BranchMode` enum instead of two `Option<String>` fields

Rather than two nullable strings (`new_branch: Option<String>`,
`reset_branch: Option<String>`), introduce an enum:

```rust
pub enum BranchMode {
    New(String),    // -b: fail if branch exists
    Reset(String),  // -B: create or reset
}
```

**Rationale**: the two options are mutually exclusive by definition; an enum
encodes that invariant in the type system rather than relying on runtime checks.
Clap's `conflicts_with` attribute on the CLI layer prevents both being supplied
simultaneously, but the library API should also reflect it.

**Alternative considered**: `Option<(BranchKind, String)>` (a single optional
field) is equivalent but slightly more verbose at call sites.

### Decision: Extend `get_worktree` with an `Option<BranchMode>` parameter; pass it into `create_slot` and `reset_slot`

Add `branch: Option<BranchMode>` to `get_worktree`, `create_slot`, and
`reset_slot`. Each function selects the right git flag based on the mode:

- `create_slot` with `Some(BranchMode::New(b))` →
  `git worktree add -b <b> <slot> <sha>`
- `create_slot` with `Some(BranchMode::Reset(b))` →
  `git worktree add -B <b> <slot> <sha>`
- `create_slot` with `None` → `git worktree add --detach <slot> <sha>`
  (unchanged)
- `reset_slot` with `Some(BranchMode::New(b))` →
  `git -C <slot> checkout -b <b> <sha>`
- `reset_slot` with `Some(BranchMode::Reset(b))` →
  `git -C <slot> checkout -B <b> <sha>`
- `reset_slot` with `None` → `git -C <slot> checkout --detach <sha>` (unchanged)

**Rationale**: `git worktree add` natively supports `-b`/`-B`, so using them
avoids a redundant subprocess for the new-slot path. For reused slots,
`git checkout -b|-B <branch> <sha>` is equally atomic — no separate detach step
is needed. Keeping the branch mode threaded through the helpers preserves the
existing separation of concerns: all git subprocess logic stays in
`worktree/mod.rs`; `main.rs` only maps CLI flags to `BranchMode`.

**Alternative considered**: provision the slot unconditionally in detached HEAD
then run a separate `git checkout -b|-B`. This works but wastes a subprocess on
the new-slot path and is strictly worse — `git worktree add` already does the
job in one call.

### Decision: Reuse an available slot regardless of branch flag

When `-b` or `-B` is supplied, the slot selection logic (find available → reset
to HEAD) runs unchanged. The branch checkout happens _after_ the slot is
prepared, the same as today's detached-HEAD checkout.

**Rationale**: the branch is a property of the worktree state, not the slot
identity. An available slot that was previously on a different branch is fine to
reuse — `git checkout -b` from that detached HEAD will start fresh.

### Decision: `Commands::Get` becomes a struct variant

```rust
Get {
    #[arg(short = 'b', value_name = "BRANCH", conflicts_with = "reset_branch")]
    new_branch: Option<String>,
    #[arg(short = 'B', value_name = "BRANCH", conflicts_with = "new_branch")]
    reset_branch: Option<String>,
},
```

Clap enforces mutual exclusion; both `new_branch` and `reset_branch` being
`None` preserves today's detached-HEAD behaviour.

## Risks / Trade-offs

- **`-b` with an existing branch name errors**: This matches `git` semantics and
  is intentional. Callers wanting idempotency should use `-B`. → _No mitigation
  needed; document clearly in CLI help text._

- **Pool reuse of a previously-branched slot**: Reusing a slot that had a branch
  checked out and is now "available" (clean working tree, no open files) is safe
  — `reset_slot` runs `git checkout -b|-B <branch> <sha>` (or `--detach <sha>`)
  which moves the slot to the new state in one step regardless of its prior
  HEAD. → _`reset_slot` handles this correctly for all three modes._

- **Default command (`bs` with no args) cannot forward flags**: `bs -b foo` will
  not work; the user must use `bs get -b foo`. This is an acceptable limitation
  given clap's default-subcommand design. → _Document in the help text._

## Migration Plan

No data-model or pool-layout changes. Existing slots are unaffected. The feature
is purely additive; omitting `-b`/`-B` produces identical behaviour to the
current release.

## Open Questions

- Should the branch name be validated (e.g. reject names containing spaces or
  other git-invalid characters) before spawning the subprocess, or should the
  error message from git itself be surfaced? **Lean toward surfacing git's
  error** — it is more authoritative and avoids maintaining a parallel
  validation rule set.
