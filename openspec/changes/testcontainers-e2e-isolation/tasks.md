## 1. Verify Dev-Dependencies

- [ ] 1.1 Confirm `testcontainers = "0.23"` is present in `[dev-dependencies]`
      in `Cargo.toml`
- [ ] 1.2 Confirm `tokio` with `macros` and `rt-multi-thread` features is in
      `[dev-dependencies]`
- [ ] 1.3 Run `mise run build` to ensure the project compiles cleanly before any
      changes

## 2. Create Test Directory Structure

- [ ] 2.1 Create `tests/common/mod.rs` — shared helper module (empty stub to
      start)
- [ ] 2.2 Create `tests/e2e_example.rs` — placeholder bootstrap test file that
      imports `common`

## 3. Implement Common Test Helpers

- [ ] 3.1 Add a `start_generic_container()` async helper in
      `tests/common/mod.rs` that starts a lightweight container (e.g., `alpine`
      or `hello-world`) using the testcontainers async API
- [ ] 3.2 Ensure the helper returns the running container handle so the caller
      owns the lifecycle (container drops when handle goes out of scope)

## 4. Implement Bootstrap Example Test

- [ ] 4.1 In `tests/e2e_example.rs`, write a `#[tokio::test]` function
      `test_container_starts_and_stops` that calls the common helper, asserts
      the container is running, and lets it drop
- [ ] 4.2 Verify the test passes locally with `cargo test --test e2e_example`
      (Docker must be running)

## 5. Verify Full Test Suite

- [ ] 5.1 Run `mise run test` and confirm both unit tests and the bootstrap E2E
      example test pass together
- [ ] 5.2 Confirm no regressions: existing `mise run test` behaviour is
      unchanged (still runs `cargo test`)

## 6. Documentation

- [ ] 6.1 Add a `## Running E2E Tests` section to `README.md` (or create one if
      absent) explaining: Docker requirement, how to run `mise run test:e2e`,
      and the convention for adding new container-backed tests
- [ ] 6.2 Add inline doc comments to `tests/common/mod.rs` explaining the helper
      contract and how to extend it
