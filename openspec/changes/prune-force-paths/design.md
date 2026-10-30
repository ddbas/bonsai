## Context

`prunePool` (`packages/core/src/pool.ts`) classifies slots via
`listWorktreesStatus` and deletes all but one `available` slot. Classification
priority is `locked` > `in use` > `available` (`classify.ts`), so the locked
check is already authoritative. The CLI in `apps/bonsai/src/program.ts` is a
thin commander wrapper.

## Goals / Non-Goals

**Goals:**

- Targeted pruning by path with `--force` for `in use` slots.
- Locked slots are unprunable by construction.

**Non-Goals:**

- A `--force` that overrides locks, or prune-all-forced without paths.
- Deleting branches or editing git bookkeeping (still delegated to
  `git worktree prune`).

## Decisions

- **New core use case `pruneSlots(deps, paths, { force })`** alongside
  `prunePool`, rather than overloading it. Reuses `listWorktreesStatus` and the
  existing delete + `worktreePrune` + failure-collection logic. The no-argument
  path stays untouched.
- **Validate first, then delete (all-or-nothing).** Paths are canonicalised
  (`canonicalOr`) and matched against pool entries; offenders are collected as
  a list of reasons and thrown as one `BonsaiError`. Rationale: a destructive
  command should not partially execute on a typo.
- **Locked check is independent of `force`.** `force` only changes the
  acceptance of `in use`; it is never consulted for `locked`. Alternative
  (force overrides lock) rejected per requirement.
- **No preserve-one in targeted mode.** The user explicitly named the slots.
- **`--force` without paths is an error**, to avoid an ambiguous "force prune
  everything" meaning.
- Reporting reuses the existing `pruned` output and failure handling in the CLI.

## Risks / Trade-offs

- Force-pruning discards uncommitted work → requires explicit `--force` and an
  explicit path; locked slots remain a protection mechanism.
- Pruning the slot containing the cwd leaves the shell in a deleted directory →
  accepted; behaves like any `rm -rf` of cwd.
