# e2e-test-infrastructure

## Purpose

Provide a reproducible, host-safe E2E and integration test infrastructure using
ephemeral Docker containers via testcontainers. Tests run fully isolated from
the host environment, execute in parallel, and are validated as part of the standard `mise run test` task.

## Requirements

### Requirement: Tests directory exists with integration test support

The project SHALL keep all E2E and integration tests that exercise the `bs`
executable in a dedicated tests directory of the CLI application, separate from
unit tests that live next to the code they test.

#### Scenario: Integration test binary compiles

- **WHEN** `mise run test` is invoked
- **THEN** all E2E/integration tests in the tests directory are discovered,
  type-check, and execute without errors

#### Scenario: Unit tests are unaffected

- **WHEN** only a package's unit tests are run
- **THEN** no container-backed E2E test is executed

### Requirement: Shared test helper module

The project SHALL provide shared test-support helpers that start isolated
containers, run git inside them, and run the `bs` executable under test against
the shared workspace. All E2E test files SHALL use these helpers.

#### Scenario: Common module is importable

- **WHEN** an E2E test file imports the shared helpers
- **THEN** it can create an isolated git environment and run `bs` without
  further setup

### Requirement: Parallel test execution by default

E2E tests SHALL be able to run in parallel. No test SHALL depend on shared
mutable state outside its own container and temporary directories unless a
documented resource conflict requires it.

#### Scenario: Two independent tests run concurrently

- **WHEN** two E2E tests are present in the suite
- **THEN** they run concurrently without interfering with one another

### Requirement: Bootstrap example test

The suite SHALL include at least one canary test that starts a container,
performs a trivial assertion, and stops the container, validating the
infrastructure end to end as part of `mise run test`.

#### Scenario: Example test passes with Docker available

- **WHEN** `mise run test` is invoked with Docker running
- **THEN** the canary test starts a container, asserts the expected condition,
  and reports success

#### Scenario: Example test failure is descriptive when Docker is absent

- **WHEN** `mise run test` is invoked without Docker
- **THEN** the test fails with a message indicating Docker is unavailable

### Requirement: Executable under test is selectable

The E2E suite SHALL run against the executable named by the `BS_BIN` environment
variable, defaulting to the built `bs` bundle. A path ending in `.js`, `.cjs` or
`.mjs` SHALL be run with the current Node executable; any other path SHALL be
executed directly.

#### Scenario: Default executable

- **WHEN** `BS_BIN` is unset and the project has been built
- **THEN** the suite runs against the built bundle

#### Scenario: Alternative executable

- **WHEN** `BS_BIN` points at another `bs` implementation
- **THEN** the same suite runs against it unchanged

### Requirement: Containers stop after each test

Each E2E test SHALL start its own container(s), and they SHALL be removed when
the test finishes, whether it passes or fails.

#### Scenario: No orphaned containers

- **WHEN** a test completes (success or failure)
- **THEN** its container is removed from the host
