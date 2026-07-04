# 🌳 Bonsai

## Running Tests

### Unit tests

```bash
mise run test          # runs cargo test (unit + integration)
```

### E2E / Integration tests

E2E tests use [testcontainers](https://crates.io/crates/testcontainers) to spin
up ephemeral Docker containers per test. They run automatically as part of
`mise run test` — no separate task needed.

**Requirements:** Docker must be running. The first run pulls images; subsequent
runs use Docker's local cache and are fast.

```bash
mise run test                        # run everything (unit + integration + E2E)
cargo test --test e2e_example        # run only the bootstrap E2E test
cargo test --test help_command       # run only the CLI help-command tests
```

### Adding new container-backed tests

1. Create `tests/<name>.rs` and declare `mod common;` at the top.
2. Call `common::start_generic_container()` (or add a new helper to
   `tests/common/mod.rs`) to get a container handle.
3. Annotate each async test with `#[tokio::test]`.
4. Let the container handle drop naturally — testcontainers removes it.

See `tests/e2e_example.rs` for a minimal working example.
