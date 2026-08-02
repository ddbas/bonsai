# cli-help-text Specification

## Purpose

Defines the standard for CLI help text: what commands, arguments, and flags must
state (purpose, usage, constraints) and what they must omit (internal
implementation rationale, mechanics, or edge-case walkthroughs), while
preserving all documented behavior.

## Requirements

### Requirement: Command help text states purpose without implementation rationale

The system SHALL present, for `bs --help` and every `bs <command> --help`, an
`about`/summary line and detail text that describe only what the command does
and how to invoke it, and SHALL NOT include explanations of internal
implementation mechanics, historical rationale, or step-by-step internal
resolution logic.

#### Scenario: Top-level help lists commands with one-line summaries

- **WHEN** the user runs `bs --help`
- **THEN** each subcommand is listed with a single-sentence summary of what it
  does, containing no multi-sentence rationale or internal mechanics

#### Scenario: Subcommand help omits internal edge-case walkthroughs

- **WHEN** the user runs `bs get --help`
- **THEN** the output describes what `bs get` does, its arguments, and their
  constraints, without a prose walkthrough of internal branch-resolution edge
  cases (e.g. how an already-checked-out-elsewhere branch is resolved
  internally)

### Requirement: Argument and flag help text states usage facts

The system SHALL present, for every documented argument and flag, help text that
states what the option does, its accepted value(s) or type, its default value
(if any), and any constraint the user must satisfy (e.g. mutual exclusivity with
another flag, or requiring another flag to be set), and SHALL NOT include prose
explaining why the option behaves that way internally.

#### Scenario: Flag help states default and accepted values

- **WHEN** the user runs `bs --help` and inspects the `--log-level` entry
- **THEN** the help text states the accepted values (`trace`, `debug`, `info`,
  `warn`, `error`) and the default (`info`), without explaining how logging is
  initialized internally

#### Scenario: Mutually exclusive flags are documented as such

- **WHEN** the user runs `bs get --help` and inspects the `-b`/`-B` and
  positional `<branch>` entries
- **THEN** each entry's help text states what it does and that it is mutually
  exclusive with the other two, without a multi-sentence explanation of the
  underlying git-mirroring rationale

#### Scenario: Flags requiring another flag are documented as such

- **WHEN** the user runs `bs get --help` and inspects the `--no-attach` entry
- **THEN** the help text states that it requires `--tmux-session` and what it
  does when set, without unrelated background on session cleanup mechanics

### Requirement: Help text changes preserve documented CLI behavior

Rewriting help text for concision SHALL NOT change any documented default value,
accepted value set, required/optional status, or mutual-exclusivity/
required-with relationship between arguments and flags.

#### Scenario: Defaults remain discoverable after text trimming

- **WHEN** a command or flag's help text is shortened
- **THEN** any default value that was documented before the change (e.g.
  `--log-level` defaulting to `info`) remains stated in the shortened text
