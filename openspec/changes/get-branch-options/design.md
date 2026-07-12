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
- After provisioning the slot (reuse or create), run the appropriate
  `git checkout -b`/`-B` command inside the slot.
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

### Decision: Extend `get_worktree` with an `Option<BranchMode>` parameter

Add `branch: Option<BranchMode>` to `get_worktree`. When `Some`, after the slot
is provisioned (reset or created), run `git -C <slot> checkout -b|-B <name>`.

**Rationale**: keeps all git orchestration inside `worktree/mod.rs`; `main.rs`
only parses the CLI flags and passes the mode down. This mirrors how
`reset_slot` and `create_slot` are already called.

**Alternative considered**: handle branch checkout entirely in `main.rs`. This
scatters git subprocess logic outside the module boundary and breaks the
existing separation of concerns.

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
  — `get_worktree` resets to detached HEAD before the branch checkout runs. →
  _Existing `reset_slot` call handles this correctly._

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
