## Why

The `bs` CLI's `--help` output (top-level and subcommand) has grown overly
verbose. Command and argument descriptions explain internal mechanics,
rationale, and edge-case behavior instead of concisely telling the reader what
the command/option does and how to use it. This makes `--help` slow to scan and
harder to use as a quick reference.

## What Changes

- Rewrite the top-level `about`/`long_about` and all subcommand `about` texts in
  `src/main.rs` to be short, focused summaries of what each command does.
- Rewrite all argument/flag doc comments (used by `clap` for `--help`) to state
  what the option does, its accepted values, and its default, without explaining
  internal implementation details or design rationale.
- Move any implementation rationale, edge-case walkthroughs, and "why it works
  this way" explanations currently embedded in doc comments out of `--help` text
  (into code comments or `README.md`/`AGENTS.md` where useful, or drop them if
  redundant).
- Preserve all factual details users need to operate the CLI: accepted values,
  default values, mutual exclusivity between flags, and required vs. optional
  status.
- No behavioral change to the CLI itself — this is a documentation/text-only
  change to `--help` output.

## Capabilities

### New Capabilities

- `cli-help-text`: Conventions for what `bs --help` and subcommand help text
  must contain (concise purpose + usage facts) and must not contain
  (implementation rationale, internal mechanics).

### Modified Capabilities

(none — no existing spec currently governs help text conventions)

## Impact

- Affected code: `src/main.rs` (clap `Parser`/`Subcommand`/`Args` derive doc
  comments and `about`/`long_about` attributes) for every subcommand (`get`,
  `list`, `current`, `open`, `lock`, `unlock`, `info`, etc.) and their
  arguments/flags.
- No changes to CLI behavior, flags, exit codes, or output formats.
- No changes to other specs' documented behavior (this only affects the `--help`
  text describing that behavior).
