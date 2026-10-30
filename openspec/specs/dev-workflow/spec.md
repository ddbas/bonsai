## Purpose

Define how contributors, git hooks and CI build, verify and install Bonsai, so
the workflow is identical locally and in CI and does not depend on a host
toolchain beyond mise.

## Requirements

### Requirement: Single task entry point

Every routine project action (build, typecheck, test, lint, format, full CI
check, install) SHALL be runnable as `mise run <task>`. Git hooks and CI SHALL
invoke only mise tasks, never the underlying build tools directly.

#### Scenario: Tasks are discoverable

- **WHEN** a contributor runs `mise tasks`
- **THEN** build, typecheck, test, lint, format, ci and install tasks are listed
  with descriptions

#### Scenario: CI uses the same entry point

- **WHEN** the CI workflow runs
- **THEN** it verifies the project by running `mise run ci`

#### Scenario: Hooks use the same entry point

- **WHEN** a commit is made
- **THEN** the pre-commit hook formats staged files and verifies the project via
  mise tasks

### Requirement: Aggregate verification task

`mise run ci` SHALL run build, typecheck, tests (including the performance SLO
test) and lint, and SHALL exit non-zero if any of them fail.

#### Scenario: All checks pass

- **WHEN** `mise run ci` runs on a clean checkout with Docker available
- **THEN** it exits zero

#### Scenario: A check fails

- **WHEN** a type error, lint error or failing test exists
- **THEN** `mise run ci` exits non-zero and identifies the failing check

### Requirement: Local install

`mise run install` SHALL build the project and install an executable `bs` into
`~/.local/bin` (or the directory in `BONSAI_INSTALL_DIR` if set), replacing any
previous `bs` there.

#### Scenario: Fresh install

- **WHEN** `mise run install` is run on a clean checkout
- **THEN** `~/.local/bin/bs` exists, is executable, and `bs help` prints the CLI
  help

#### Scenario: Re-install replaces previous binary

- **WHEN** `mise run install` is run when `~/.local/bin/bs` already exists
- **THEN** it is replaced with the newly built executable

### Requirement: Single toolchain definition

The Node.js version SHALL be defined once, in `.nvmrc`, and mise SHALL honour
it. Markdown, TypeScript and JSON SHALL be linted and formatted by one tool
(Biome).

#### Scenario: Node version resolved from .nvmrc

- **WHEN** `mise exec -- node --version` is run in the repository
- **THEN** it prints the version in `.nvmrc`

#### Scenario: Markdown formatting is checked

- **WHEN** a Markdown file is badly formatted and `mise run lint` runs
- **THEN** it fails and the file is reported

### Requirement: No release pipeline

The project SHALL NOT publish release artifacts. Distribution is by cloning the
repository and running `mise run install`.

#### Scenario: No release workflow

- **WHEN** the repository is inspected
- **THEN** no workflow or task creates tags, tarballs or GitHub releases
