## Purpose

Enable structured file-based logging in the bonsai CLI with sensible defaults,
platform-aware directory resolution, daily log rotation, retention management,
and resilience to logging failures.

## Requirements

### Requirement: Default log level

The system SHALL default to the `info` log level when no override is provided,
without requiring any environment variable or configuration file.

#### Scenario: Running a command without any log-level override

- **WHEN** the user runs `bs get` (or any subcommand) without passing
  `--log-level`
- **THEN** log events at `info`, `warn`, and `error` severity are written to the
  log file, and events at `debug`/`trace` severity are not

### Requirement: Logs are file-only and never printed to the terminal

The system SHALL write all log output exclusively to a log file and SHALL NOT
write log output to stdout or stderr under any configuration.

#### Scenario: Command produces log events during normal operation

- **WHEN** a bonsai command runs and emits `info`-level (or higher) log events
  internally
- **THEN** none of that log output appears in the process's stdout or stderr,
  and the command's normal output (paths, tables, status messages) is unaffected
  in content and formatting

#### Scenario: Logging initialization fails

- **WHEN** the log directory or log file cannot be created or opened (e.g.
  permissions error, read-only filesystem)
- **THEN** the CLI command still completes and produces its normal stdout/stderr
  output and exit code, without crashing or printing logging-internal errors to
  stdout

### Requirement: Logs are routed to the platform-appropriate log directory

The system SHALL write log files under the OS-appropriate XDG/platform log or
state directory for the `bonsai` application, resolved at runtime rather than
hardcoded to a fixed path.

#### Scenario: Log directory does not yet exist

- **WHEN** a bonsai command runs for the first time on a machine and the
  platform log directory for bonsai does not yet exist
- **THEN** the system creates the directory (and any missing parent directories)
  before writing the first log file into it

#### Scenario: Resolving the log directory on Linux

- **WHEN** the system resolves the log directory on Linux
- **THEN** it uses `$XDG_STATE_HOME/bonsai/logs` if `XDG_STATE_HOME` is set,
  otherwise `~/.local/state/bonsai/logs`

### Requirement: Global `--log-level` override flag

The system SHALL expose a global CLI flag that allows the user to override the
default log level, accepted on the top-level `bs` command and applied
consistently regardless of which subcommand is invoked.

#### Scenario: User overrides the log level to debug

- **WHEN** the user runs `bs --log-level debug get`
- **THEN** log events at `debug`, `info`, `warn`, and `error` severity are
  written to the log file for that invocation, and `trace`-level events are not

#### Scenario: User provides an invalid log level value

- **WHEN** the user runs `bs --log-level bogus get`
- **THEN** the CLI exits with a standard argument-parsing error identifying the
  accepted values, and does not attempt to run the `get` command

#### Scenario: Flag omitted

- **WHEN** the user runs any command without `--log-level`
- **THEN** the effective log level is `info` (the documented default)

### Requirement: Log files rotate and retention is bounded

The system SHALL rotate log files on a fixed daily schedule and SHALL prune log
files older than a documented retention window so log storage does not grow
unbounded.

#### Scenario: A new day's log file is created

- **WHEN** a bonsai command runs on a calendar day for which no log file yet
  exists in the log directory
- **THEN** a new dated log file is created for that day and subsequent log
  events for that day are appended to it

#### Scenario: Old log files are pruned beyond the retention window

- **WHEN** the number of existing daily log files for bonsai in the log
  directory exceeds the documented retention count
- **THEN** the oldest log file(s) beyond the retention window are deleted so at
  most the documented number of daily log files remain

### Requirement: Logging failures never affect command behavior or exit status

The system SHALL treat all logging operations (initialization, writing,
rotation, retention pruning) as best-effort side effects that never alter a
command's stdout, stderr, or exit code.

#### Scenario: Log write fails mid-command (e.g. disk full)

- **WHEN** a bonsai command is running and a log write fails after logging was
  successfully initialized
- **THEN** the command continues executing and completes with the same result
  and exit code it would have produced if the log write had succeeded
