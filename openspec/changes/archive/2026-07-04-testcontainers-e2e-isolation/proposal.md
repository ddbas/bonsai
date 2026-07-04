## Why

The project currently has no E2E or integration tests, and running such tests
against real services risks polluting or impacting the developer's host
environment. Using `testcontainers` provides fully isolated, ephemeral
containers per test run — already listed as a dev-dependency — so every test
starts from a clean slate without manual setup.

## What Changes

- Add an E2E / integration test layer under `tests/` that spins up required
  services via testcontainers.
- Configure Tokio's async test runtime (already a dev-dependency) as the
  executor for all container-backed tests.
- Enable parallel test execution so the suite stays fast even as it grows.
- Document the testing conventions so contributors know how to add new E2E
  tests.

## Capabilities

### New Capabilities

- `e2e-test-infrastructure`: Foundational setup for container-backed
  E2E/integration tests — directory layout, shared helpers, Tokio runtime
  config, and parallel execution strategy.

### Modified Capabilities

<!-- No existing spec-level requirements are changing. -->

## Impact

- **`Cargo.toml`**: `testcontainers` and `tokio` dev-dependencies already
  present; may need version pins or feature flags confirmed.
- **`tests/`**: New directory created; all integration tests live here to
  benefit from Cargo's test-binary isolation.
- **`mise.toml`**: No changes — E2E tests run as part of the existing
  `mise run test` task (`cargo test`).
- No production (`src/`) code changes required.
- No breaking changes.
