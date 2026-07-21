## ADDED Requirements

### Requirement: `bs info` subcommand reports bonsai's runtime paths and metadata

The system SHALL provide an `info` subcommand that prints, to stdout, the
following fields: bonsai version, the effective log level for the invocation,
the resolved log directory path, the resolved path to the current day's log
file, and the bonsai managed root path (`~/.bonsai`).

#### Scenario: Running `bs info` with default settings

- **WHEN** the user runs `bs info` without any global flags
- **THEN** the command prints all five fields (version, log level, log
  directory, current log file, managed root) to stdout and exits with status 0

#### Scenario: Effective log level reflects an override

- **WHEN** the user runs `bs --log-level debug info`
- **THEN** the printed log level field shows `debug`, not the default `info`

### Requirement: `bs info` output is plain, stable, and greppable

The system SHALL print `bs info` output as one field per line in a stable
`key: value` text format, using the same tilde-abbreviated path style already
used by `bs current`/`bs list` for path values.

#### Scenario: Path values are tilde-abbreviated

- **WHEN** a reported path (log directory, current log file, or managed root)
  lies under the user's home directory
- **THEN** it is printed with the home directory prefix replaced by `~`,
  consistent with `bs current`'s path formatting

#### Scenario: Output is script-friendly

- **WHEN** a script pipes `bs info` output through `grep 'log directory'`
- **THEN** exactly one line matches, containing the resolved log directory path,
  without requiring a JSON parser

### Requirement: `bs info` reports resolved paths regardless of filesystem state

The system SHALL report the log directory, current log file, and managed root
paths that bonsai resolves and would use, without requiring those paths to
already exist on disk, and without performing any write or
logging-initialization side effects itself.

#### Scenario: Log directory has never been created

- **WHEN** the user runs `bs info` on a machine where the log directory has not
  yet been created (e.g. logging previously failed to initialize due to
  permissions, or no bonsai command has run yet)
- **THEN** `bs info` still prints the log directory and current log file paths
  it would use, and exits with status 0 rather than erroring

#### Scenario: `bs info` does not create files or directories

- **WHEN** the user runs `bs info`
- **THEN** no new files or directories are created on disk as a side effect of
  running the command
