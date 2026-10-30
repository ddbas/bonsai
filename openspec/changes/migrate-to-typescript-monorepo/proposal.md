## Why

Bonsai is a Rust/cargo project, but the author's other tools (`agentctl`,
`actual-uploader`) are pnpm + Turborepo + TypeScript monorepos with shared
tooling and conventions. Moving Bonsai onto the same stack lets all projects
share one toolchain, one set of agent/CI conventions, and one hexagonal
architecture.

## What Changes

- Replace the Rust implementation of `bs` with a TypeScript implementation in a
  pnpm + Turborepo monorepo, using the latest versions of all tools (exact
  pinned): `apps/bonsai` (thin CLI) plus `packages/core`, `packages/git`,
  `packages/tmux`, `packages/logging` (hexagonal: core + ports and adapters).
- **BREAKING**: `bs` now requires Node.js at runtime (single-file bundle
  `cli.cjs`). Release tarballs are no longer produced; install is `git clone` +
  `mise run install`.
- Port all Rust integration tests to vitest + testcontainers, first validating
  them against the Rust binary, then running them unchanged against the
  TypeScript bundle. CLI behaviour is otherwise unchanged.
- Replace the criterion benchmark with a vitest performance test enforcing the
  same SLO thresholds.
- Replace Prettier with Biome (lint + format, including Markdown). Pin Node via
  `.nvmrc`.
- Standardise tooling: all main tasks are mise tasks that call `turbo run`;
  lefthook and CI call only `mise run`.
- Remove Cargo files, Rust sources/tests/benches, cargo CI jobs, the release
  workflow and task, and the `rust-*` agent skills. Install the `turborepo`
  agent skill.
- Delivered as a single PR.

## Capabilities

### New Capabilities

- `dev-workflow`: how contributors and CI build, test, lint, format and install
  the project (task entry points, hooks, install result, supported toolchain).

### Modified Capabilities

- `e2e-test-infrastructure`: Rust/Cargo/Tokio-specific requirements become
  toolchain-neutral container-isolated test requirements (vitest, selectable
  binary under test).
- `worktree-list`: the performance SLO requirement no longer names the Rust
  benchmark or script; thresholds are unchanged.

## Impact

- Everything under `src/`, `tests/`, `benches/`, `Cargo.*`,
  `scripts/check-bs-ls-perf.sh` is replaced or removed.
- `mise.toml`, `lefthook.yml`, `.github/workflows/*`, `.mise/tasks/release`,
  `.prettierrc.json`, `.prettierignore`, `.gitignore`, `skills-lock.json`,
  `.agents/skills/rust-*`, `openspec/config.yaml` change.
- New root config: `.nvmrc`, `package.json`, `pnpm-workspace.yaml`,
  `turbo.json`, `tsconfig.base.json`, `biome.json`, `mise.lock`.
- User-facing CLI behaviour (all `openspec/specs/*` other than the two modified
  above) must remain identical.
