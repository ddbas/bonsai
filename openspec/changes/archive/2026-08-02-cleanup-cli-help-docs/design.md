## Context

`bs`'s CLI surface is defined in `src/main.rs` via `clap`'s derive API. The
top-level `Cli` struct, the `Commands` enum (`Get`, `List`, `Current`, `Help`,
`Lock`, `Unlock`, `Info`), and each command's `Args`-style fields carry doc
comments that `clap` renders verbatim as `--help` text (short line = summary
shown in the command list / `about`; full doc comment = `long_about` shown under
`bs <cmd> --help`).

Today several of these doc comments read like design notes: they explain _why_
`bs get` mirrors `git checkout -b`/`-B` semantics, walk through the
already-checked-out-elsewhere edge case for positional `<branch>`, describe
internal resolution order, etc. This is useful context for a contributor reading
the source, but it clutters the interactive `--help` a user reaches for when
trying to recall a flag's default or accepted values.

## Goals / Non-Goals

**Goals:**

- Every `about`/doc-comment string rendered by `clap` states only: what the
  command/option does, its accepted value(s)/type, its default (if any), and any
  hard constraint the user must know before invoking it (e.g. mutual
  exclusivity, a required companion flag).
- Preserve every fact currently discoverable via `--help` (defaults, mutually
  exclusive flags, required-with relationships, exit-status notes that affect
  usage) — only the framing/rationale/mechanics prose is cut.
- Keep short summaries (the first line, used in the command list and as `about`)
  to a single sentence.
- Establish a lightweight, reusable convention (`cli-help-text` spec) so future
  commands/flags are added with the same level of concision.

**Non-Goals:**

- No change to CLI behavior, flag names, exit codes, or output formatting.
- No restructuring of the command tree (no renames, no new commands).
- Not moving detailed rationale into `README.md` unless it would otherwise be
  lost and is worth preserving somewhere (case-by-case, not required).
- Not touching `tracing`/log-level _values_ or their semantics — only the
  doc-comment text describing them.

## Decisions

- **Keep `about` as the short line, `long_about` (i.e. the full doc comment) for
  details.** clap derives `about` from the first paragraph and `long_about` from
  the whole comment when `long_about = None` is used (as today). We keep this
  mechanism rather than manually splitting `about`/ `long_about`, to minimize
  diff surface and stay idiomatic to clap-derive.
- **Trim in place, one command/arg at a time**, rather than rewriting
  `main.rs`'s CLI definition from scratch. Lower risk of introducing behavioral
  drift (e.g. accidentally changing a `conflicts_with`/`requires` attribute)
  since only comment text changes.
- **Rule of thumb for what stays vs. goes**: keep if it answers "what does this
  do / what can I pass / what's the default / what else must I set"; cut if it
  answers "why was it built this way / what happens internally / what's the
  historical git-mirroring rationale" unless that phrasing is the most concise
  way to state a _user-facing_ constraint (e.g. "mirrors `git checkout -b`" is
  acceptable shorthand for behavior, not rationale).
- **No automated tests for help text content.** `--help` text is documentation
  prose; correctness is verified via manual review of `bs --help` and
  `bs <cmd> --help` output during review, not via unit tests. (Existing
  `#[cfg(test)]` tests in `main.rs` are unrelated to help text and are left
  untouched.)

## Risks / Trade-offs

- **[Risk] Trimming could accidentally drop a fact a user relies on** (e.g. a
  default value or a `conflicts_with` relationship) → **Mitigation**: after
  editing, manually run `bs --help` and `bs <cmd> --help` for every changed
  command and diff mentally against the previous output to confirm all
  values/defaults/constraints are still present, just shorter.
- **[Risk] Over-trimming could make `--help` too terse to explain non-obvious
  behavior** (e.g. `bs get` being the implicit default command, or `-b`/`-B`
  mirroring `git checkout -b`/`-B`) → **Mitigation**: keep one-line behavioral
  facts that are load-bearing for correct usage; only cut multi-paragraph
  walkthroughs of edge cases and internal resolution logic.
- **[Trade-off] Some contributor-facing rationale currently in doc comments will
  no longer be visible via `--help`** → acceptable since `--help` is a
  user-facing reference, not contributor documentation; rationale can live in
  regular (non-doc) code comments or `openspec/specs/` where it's still
  discoverable by contributors.
