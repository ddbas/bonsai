## Project Tasks (mise)

This project uses **[mise](https://mise.jdx.dev/)** as the task runner and tool
version manager. Always run project tasks via `mise run` rather than invoking
`cargo`, `npm`, or other tools directly — this ensures the correct tool versions
(defined in `mise.toml`) are active.

```bash
mise run build           # Build the project (cargo build)
mise run build:release   # Build in release mode
mise run run             # Run the project
mise run test            # Run the test suite
```

All new features MUST include unit and/or integration tests.

## Integration & E2E Test Isolation (REQUIREMENT)

Any test that exercises the filesystem, git repositories, home-directory
behaviour, or any command that shells out to `git` **MUST** use
[testcontainers](https://crates.io/crates/testcontainers) for isolation. Tests
MUST NOT create git repos or write files directly on the host machine using bare
`TempDir::new()` + `Command::new("git")` setups.

### Why

`bs get` manipulates `~/.bonsai` and calls `git worktree` commands. Without
container isolation a test run can:

- Pollute `~/.bonsai` or the developer's own git repos.
- Be affected by the host's global `~/.gitconfig`, commit hooks, or
  `GIT_DIR`/`GIT_INDEX_FILE` env vars (especially inside `pre-commit` hooks).
- Produce non-deterministic results across machines with different git versions.

### Rules for agents

- **ALL** integration/E2E tests that touch git or the filesystem MUST use the
  `GitEnv` helper in `tests/common/mod.rs`.
- `GitEnv::new().await` starts a pinned `alpine/git` Docker container that
  provides a clean git installation (no global config, no hooks, no leaked env
  vars). The container's `/workspace` is bind-mounted to a host `TempDir` so
  that the `bs` host binary can operate on the same files.
- All git _setup_ and _mutation_ operations (init, config, commit, etc.) MUST
  run via `env.git(&[...]).await` (container exec), not via host git.
- `BONSAI_ROOT` MUST always be pointed at `env.bonsai_path` (a `TempDir`
  separate from `~/.bonsai`).
- Every test function that uses `GitEnv` MUST be annotated `#[tokio::test]`.
- See `openspec/specs/e2e-test-infrastructure/spec.md` for the full spec.

Available tasks are defined in `mise.toml`. To list them:

```bash
mise tasks
```
