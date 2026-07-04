## Context

The project (`bonsai` / `bs` CLI) currently has no integration or E2E tests. The
`testcontainers = "0.23"` and `tokio` crates are already declared as
dev-dependencies in `Cargo.toml`, but no test code uses them. The goal is to
establish the scaffolding and conventions so future E2E tests run in ephemeral
containers, are fully isolated from the host, and execute as fast as possible.

## Goals / Non-Goals

**Goals:**

- Create a reproducible, host-safe test infrastructure using testcontainers.
- Run E2E/integration tests with full async Tokio support.
- Execute tests in parallel by default (no global serialisation).
- E2E/integration tests run as part of the existing `mise run test` task — no
  separate task needed.
- Establish a minimal working example test so the scaffold can be verified.
- Document conventions for adding new container-backed tests.

**Non-Goals:**

- Migrating or rewriting existing unit tests.
- Adding test coverage beyond the infrastructure bootstrap example.
- CI/CD pipeline changes (out of scope for this change).
- Supporting non-Docker container runtimes at this time.

## Decisions

### 1. `tests/` directory for integration tests (not inline `#[cfg(test)]`)

Cargo compiles each file under `tests/` as its own test binary. This gives:

- True process-level isolation between integration test modules.
- Clean separation from unit tests (which live in `src/`).
- Individual test binaries can be run with `cargo test --test <name>`.

**Alternative considered**: inline `#[cfg(test)]` modules. Rejected because they
share the same binary as the library, which complicates isolation and makes
selective E2E execution harder.

### 2. One test file per logical concern, sharing a `common` helper module

`tests/common/mod.rs` will hold container startup helpers and shared fixtures.
Individual test files (`tests/e2e_*.rs`) import from `common`. This avoids
duplicating setup code while keeping test files focused.

**Alternative considered**: A single `tests/integration.rs` monolith. Rejected
because it grows unbounded and forces serial file-level compilation.

### 3. Tokio `#[tokio::test]` for async tests; rely on Tokio's thread-pool for parallelism

`testcontainers` async API requires an async runtime. `tokio::test` spawns a
fresh runtime per `#[test]`-annotated function, enabling:

- Each test function runs in its own Tokio runtime → no cross-test state
  leakage.
- Cargo's default test harness runs test functions in parallel threads.

**Alternative considered**: `async-std`. Rejected — `tokio` is already a
dev-dependency and is the de-facto standard for Rust async.

### 4. No global `OnceCell`/`lazy_static` container for shared instances

Each test function starts and stops its own container. This maximises isolation
at a small overhead cost. For the current project size this is acceptable; if
test suites grow large a shared-per-module fixture can be introduced later.

**Alternative considered**: Sharing one container across all tests via
`OnceLock`. Rejected for now because it requires serialising tests that touch
the same container and makes teardown non-deterministic.

### 5. Integration tests run under the existing `mise run test` task

`mise run test` already executes `cargo test`, which compiles and runs all test
binaries including those under `tests/`. No new task is needed. Unit tests and
integration tests run together in one invocation, keeping the developer workflow
simple.

## Risks / Trade-offs

- **Docker required at runtime** → Tests will fail if Docker is not available.
  Mitigation: document the requirement; tests are skipped gracefully by
  testcontainers when no daemon is found (it panics with a clear message).
- **Container pull latency on first run** → Images are cached by Docker locally
  after first pull, so subsequent runs are fast. Mitigation: document that first
  run may be slow; CI should use a layer cache.
- **Parallel container startup load** → Many tests starting containers
  simultaneously could exhaust system resources on low-memory machines.
  Mitigation: keep container count low; add `#[serial]` attribute (via
  `serial_test` crate) only if contention is observed in the future.
- **testcontainers 0.23 API stability** → The crate is pre-1.0. Mitigation: pin
  the version in `Cargo.toml`; upgrades are a deliberate opt-in.

## Migration Plan

1. No existing tests to migrate.
2. Add `tests/` directory and helpers.
3. Add `mise run test:e2e` task.
4. Validate with `mise run test:e2e` locally (requires Docker).
5. `mise run test` runs both unit and integration tests in one invocation — no
   new task added.

## Open Questions

- What services will real E2E tests need (databases, message brokers, etc.)? The
  bootstrap example uses a generic `alpine` container as a placeholder — real
  service images will be chosen per feature.
