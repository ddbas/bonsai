<div align="center">

# 🌳 Bonsai

<h3 align="center">Instant, clean git worktrees on demand.</h3>

Bonsai manages a pool of git worktrees so you can jump between tasks without
ever stashing, committing half-finished work, or waiting for a fresh clone. Run
`bs` and you get a clean slot at the current HEAD — ready to use, already on
disk. When you're done, put it back in the pool; the next call reuses it in
milliseconds.

</div>

## Quick Start

```
$ bs get
🌳 ~/.bonsai/myrepo/a1b2c3d4
```

```
$ bs list
available  ~/.bonsai/myrepo/a1b2c3d4
in use     ~/.bonsai/myrepo/b5c6d7e8 (main)      ⚙2
in use     ~/.bonsai/myrepo/c9d0e1f2 (my-feature)
```

```
$ bs help
```

## Install

> **TBD** — installation instructions coming soon.

## Prerequisites

The following tools must be available on `PATH` at runtime:

| Tool   | Purpose                                           | Install (macOS)     |
| ------ | ------------------------------------------------- | ------------------- |
| `git`  | Worktree management                               | pre-installed       |
| `lsof` | Detect open file handles in pool slot directories | `brew install lsof` |

## Running Tests

```bash
mise run test          # run everything (unit + integration + E2E)
```

E2E tests use [testcontainers](https://crates.io/crates/testcontainers) to spin
up ephemeral Docker containers per test. **Docker must be running.** The first
run pulls images; subsequent runs use Docker's local cache and are fast.

```bash
cargo test --test e2e_example    # run only the bootstrap E2E test
cargo test --test help_command   # run only the CLI help-command tests
```

### Adding container-backed tests

1. Create `tests/<name>.rs` and declare `mod common;` at the top.
2. Call `common::start_generic_container()` (or add a helper to
   `tests/common/mod.rs`) to get a container handle.
3. Annotate each async test with `#[tokio::test]`.
4. Let the container handle drop naturally — testcontainers removes it.

See `tests/e2e_example.rs` for a minimal working example.
