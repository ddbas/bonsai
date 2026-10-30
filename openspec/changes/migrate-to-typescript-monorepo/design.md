## Context

Bonsai (`bs`) is ~3.1k lines of Rust (`src/main.rs` clap CLI,
`src/worktree/mod.rs` 2k lines mixing pure rules with `git` subprocess calls,
`src/worktree/branch_state.rs`, `src/tmux.rs`, `src/logging.rs`) and ~4.6k lines
of Rust integration tests backed by a Docker `GitEnv` helper. Reference repos
(read-only): `~/repos/agentctl` (hexagonal layout, rspack single-file CLI
bundle, testcontainers e2e driven by an env var for the binary, Biome boundary
rule) and `~/repos/actual-uploader` (Biome with Markdown, mise-only CI,
`mise.lock`). See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**

- Behaviour-identical `bs` in TypeScript, in a pnpm + Turborepo monorepo with
  hexagonal boundaries enforced by lint.
- Prove parity by running one e2e suite against both implementations.
- One tooling path: lefthook/CI → `mise run` → `turbo run`.

**Non-Goals:**

- New features or CLI behaviour changes.
- Publishing (npm, releases, Node SEA binaries).
- Changing other specs' requirements.

## Decisions

**Layout (full hexagonal).** `packages/core` (domain, use cases, port interfaces
`Git`/`Tmux`/`Logger`; no `node:*`, no adapters, no pino/commander),
`packages/git`, `packages/tmux`, `packages/logging` (sole implementations of the
ports, no business logic), `apps/bonsai` (commander wiring + composition root
only). Biome `noRestrictedImports` on `packages/core` enforces it.
_Alternative:_ single package — rejected, user wants full hexagonal like
agentctl.

**Oracle-first migration.** Port the e2e tests first and make them pass against
the Rust binary via `BS_BIN`; only then write the TS code and run the same tests
against `cli.cjs`. The tests become the behaviour spec. _Alternative:_ port code
then tests — rejected, no independent parity proof.

**Crate → package mapping.** clap→commander, owo-colors→picocolors,
tracing(+appender/subscriber)→pino 10.4.0, dirs→`os.homedir()`,
uuid/rand→`crypto.randomUUID()`, chrono→`Date`, anyhow→typed errors,
testcontainers (rust)→testcontainers (node), tokio/tempfile dropped.

**Bundling.** Rspack builds `apps/bonsai/dist/cli.cjs` (shebang, mode 755,
version injected from `package.json`), as in agentctl. `mise run install` copies
it to `~/.local/bin/bs`. Node is required at runtime. _Alternatives:_ Node SEA
binary / npm publish — deferred, not needed now.

**Toolchain.** Latest versions, exact-pinned: TypeScript 7, turbo, vitest, Biome
≥2.6 (lint + format incl. Markdown with `proseWrap: always`; replaces Prettier),
pnpm, rspack, commander, picocolors, pino, testcontainers. Node version lives
only in `.nvmrc`; `mise.toml` sets
`idiomatic_version_file_enable_tools = ["node"]` and has no node pin.
`mise.lock` committed. pnpm hardening from agentctl (`minimumReleaseAge`,
`allowBuilds`).

**Task flow.** mise tasks
`build typecheck test lint format ci install openspec:open` call `turbo run`
directly (no `pnpm run` hop). Turbo tasks: `build`, `typecheck`, `test`
(dependsOn build), `ci` aggregate, root `//#lint`/`//#format:biome` with
explicit `inputs`; `passThroughEnv` for `BS_BIN`, `DOCKER_HOST`,
`TESTCONTAINERS_*`. lefthook and CI call only `mise run`. Pre-commit runs a
single turbo-ordered task so build→test runs once. `format` is a mise task
accepting staged files, used with `stage_fixed`.

**Async git.** The git adapter uses async `execFile` with bounded parallelism
for per-slot status/branch-state (Rust was serial).

**Performance SLO.** criterion bench + shell script replaced by a vitest perf
test on core's classification path using a fake git port that simulates per-slot
cost, same scenarios and thresholds (locked/dirty p95@50 ≤ 100ms; scaling ≤ 6x
locked, ≤ 15x dirty/available). Runs in `mise run test`.

**Biome scope.** Ignore `.agents`, `.pi`, `.opencode`, `.codegraph`, `.mise`,
`dist`, `node_modules`, and `openspec/` (as agentctl does) so OpenSpec tooling
output is not reformatted. Repo Markdown (README, TODO) is formatted once.

**Docs/AGENTS.md.** Root and per-package `AGENTS.md` (mirroring agentctl) are
desirable but MUST NOT be created without explicit user permission; ask during
implementation.

## Risks / Trade-offs

- [Node cold start (~40ms+) vs native] → measure `bs list` on a 5-slot pool;
  accept unless used in prompts.
- [Behavioural drift in git edge cases] → oracle-first e2e suite; any justified
  test change is called out in the PR.
- [Tests encoding Rust internals (`classification_agreement.rs`, inline
  `#[cfg(test)]`)] → port intent as core unit tests.
- [Perf SLO calibration differs in Node] → thresholds target the classification
  path with a fake git port, as before; changing thresholds requires a spec
  update and user approval.
- [Biome Markdown ~92% Prettier-compatible] → one-time reformat, reviewed in PR.
- [Docker required for pre-commit tests] → already true today.
- [Single large PR] → organised as ordered commits following the task groups.

## Migration Plan

Single PR on `migrate-to-ts`, commits ordered as task groups 1–10. Rust is
deleted only after the TS bundle passes the full e2e suite. Rollback: revert the
PR (history retains the Rust implementation).
