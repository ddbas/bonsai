## 1. Scaffold monorepo root

- [x] 1.1 Add `.nvmrc` (Node 26.x as in agentctl) and set
      `idiomatic_version_file_enable_tools = ["node"]` in `mise.toml`; verify
      `mise exec -- node --version` matches `.nvmrc`
- [x] 1.2 Add root `package.json` (private, ESM, exact-pinned latest
      typescript/turbo/vitest/@biomejs/biome/@types/node, pnpm `packageManager`)
      and `pnpm-workspace.yaml` (`apps/*`, `packages/*`, `minimumReleaseAge`,
      `allowBuilds`); verify `pnpm install` succeeds
- [x] 1.3 Add `tsconfig.base.json` (strict, `noUncheckedIndexedAccess`,
      `verbatimModuleSyntax`, ES2023, Bundler resolution) and `turbo.json`
      (build/typecheck/test/ci tasks with inputs/outputs, `passThroughEnv`,
      `//#lint`, `//#format:biome`); verify `turbo run build` runs on the empty
      workspace
- [x] 1.4 Add `biome.json` (vcs ignore file, organizeImports assist,
      `noExplicitAny` error, Markdown formatter `proseWrap: always`, ignore
      `.agents .pi .opencode .codegraph .mise dist node_modules openspec`);
      verify `biome check .` passes
- [x] 1.5 Rewrite `mise.toml` tools/tasks
      (`build typecheck test lint format ci install openspec:open` calling
      `turbo run`; drop rust, cargo-edit, pandoc, prettier, cargo/bench tasks,
      node pin) and commit `mise.lock`; verify `mise tasks` lists them and
      `mise run lint` passes
- [x] 1.6 Rewrite `lefthook.yml` to call only `mise run` (format with
      `stage_fixed`, single turbo-ordered verification, openspec validate via
      `scripts/openspec-validate-staged.sh`); verify a test commit runs the
      hooks
- [x] 1.7 Update `.gitignore` (node_modules, dist, .turbo, coverage, mise lock
      layout) and run `npx skills add vercel/turborepo`; verify
      `skills-lock.json` lists `turborepo`

## 2. Package skeletons and boundaries

- [x] 2.1 Create `packages/core`, `packages/git`, `packages/tmux`,
      `packages/logging` (`@bonsai/*`, exports `./src/index.ts`, typecheck/test
      scripts, tsconfig) and `apps/bonsai` mirroring agentctl; verify
      `mise run typecheck` passes
- [x] 2.2 Add `apps/bonsai/rspack.config.ts` (builtin swc loader, shebang
      banner, chmod 755, version DefinePlugin → `dist/cli.cjs`) and
      `vitest.config.ts` (120s timeouts); verify `mise run build` emits an
      executable `cli.cjs`
- [x] 2.3 Add Biome `noRestrictedImports` override for `packages/core` (no
      adapters, pino, commander, `node:*`); verify a temporary `node:fs` import
      in core fails lint, then revert
- [ ] 2.4 Define port interfaces (`Git`, `Tmux`, `Logger`, clock/fs as needed)
      in `packages/core/src/ports.ts`; verify typecheck passes
- [ ] 2.5 Ask the user for permission to create root and per-package `AGENTS.md`
      (mirroring agentctl); create them only if approved

## 3. E2E test infrastructure and oracle

- [x] 3.1 Port `tests/common/mod.rs` (`GitEnv`, container helpers) to
      `apps/bonsai/tests/e2e/support/` with a test-image Dockerfile and `BS_BIN`
      resolution (script paths run via node); verify the canary test (port of
      `e2e_example.rs`) passes with Docker running
- [x] 3.2 Port `worktree_get.rs`, `worktree_get_tmux.rs`,
      `worktree_copy_ignored_files.rs` to vitest; verify they pass with `BS_BIN`
      = the Rust binary
- [x] 3.3 Port `worktree_list.rs`, `worktree_status.rs`,
      `worktree_branch_state.rs`; verify they pass against the Rust binary
- [x] 3.4 Port `worktree_lock.rs`, `worktree_prune.rs`, `info_command.rs`,
      `help_command.rs`, `logging.rs`; verify they pass against the Rust binary
- [x] 3.5 Record Rust tests that are not black-box
      (`classification_agreement.rs`, inline `#[cfg(test)]` in `src/`) in a
      checklist for task group 4; verify every `tests/*.rs` file is either
      ported or listed

## 4. Core

- [ ] 4.1 Port pure types and classification (`classify`, slot status,
      `tilde_path`, slot naming, locked-line parsing) to `packages/core` with
      unit tests; verify tests cover locked > in use > available priority
- [ ] 4.2 Port branch-state logic (`branch_state.rs`: default-ref selection,
      merged/squash/rebase detection) with unit tests for each README "When is a
      slot available?" case; verify tests pass
- [ ] 4.3 Implement use cases (get, list, status, lock/unlock, prune incl.
      preserve-one-available, info, copy-ignored-files) against ports with unit
      tests using fakes; verify tests pass
- [ ] 4.4 Port the intent of `classification_agreement.rs` and remaining inline
      Rust unit tests from the 3.5 checklist; verify checklist is empty

## 5. Adapters

- [ ] 5.1 Implement `packages/git` (async `execFile`, bounded parallelism, git
      env handling per `git_cmd()`) with parser unit tests; verify typecheck and
      tests pass
- [ ] 5.2 Implement `packages/tmux` (port `src/tmux.rs`, `BONSAI_TMUX_SOCKET`,
      detached sessions) with unit tests; verify tests pass
- [ ] 5.3 Implement `packages/logging` with pino 10.4.0 (levels, log file
      location/format per `src/logging.rs`); verify unit tests pass

## 6. CLI and parity

- [ ] 6.1 Implement commander program and composition root in `apps/bonsai/src`
      (all subcommands/options, exact help text, picocolors output,
      `BONSAI_ROOT`); verify `program.test.ts` passes
- [ ] 6.2 Build `dist/cli.cjs` and run the full e2e suite with default `BS_BIN`;
      verify it passes with no weakened assertions (justify and flag any
      exception)
- [ ] 6.3 Measure `bs list` startup on a 5-slot pool and report the cold-start
      figure to the user

## 7. Performance SLO

- [ ] 7.1 Implement a vitest perf test on core classification with a fake git
      port covering all-locked, all-dirty and all-available scenarios at 5 and
      50 slots, enforcing the thresholds in the `worktree-list` spec; verify it
      passes and fails when a threshold is artificially lowered

## 8. CI, hooks, docs, OpenSpec

- [ ] 8.1 Replace `.github/workflows/ci.yml` with one job (concurrency, timeout,
      `jdx/mise-action` with cache, `.turbo` cache, `mise run ci`); delete
      `.github/workflows/release.yml` and `.mise/tasks/release`; verify CI is
      green on the PR
- [ ] 8.2 Remove `.prettierrc.json`/`.prettierignore`; run Biome format on repo
      Markdown; verify `mise run lint` passes
- [ ] 8.3 Update `openspec/config.yaml` context
      (TypeScript/Node/pnpm/Turborepo/Rspack/Biome/vitest, binary `bs`,
      conventional commits); verify `openspec validate --all --no-interactive`
      passes
- [ ] 8.4 Check README for cargo/Rust wording and adjust only what is required
      (ask before other doc edits); verify `grep -ri "cargo" README.md` is empty

## 9. Remove Rust

- [ ] 9.1 Delete `Cargo.toml`, `Cargo.lock`, `src/`, Rust files under `tests/`,
      `benches/`, `scripts/check-bs-ls-perf.sh`, `target/`; verify `mise run ci`
      still passes
- [ ] 9.2 Remove `.agents/skills/rust-*` (and `skills-lock.json` / mirrored
      entries); verify
      `grep -rniE "cargo|clippy|rustfmt" . --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=archive`
      shows no stale references

## 10. Final verification

- [ ] 10.1 From a clean state (`rm -rf node_modules dist .turbo`), run
      `mise install`, `pnpm install --frozen-lockfile` via mise, and
      `mise run ci`; verify all pass
- [ ] 10.2 Run `mise run install` and smoke-test `~/.local/bin/bs` (`get`,
      `list`, `status`, `lock`) in a scratch git repo
- [ ] 10.3 Push the branch and open a single PR following the
      create-pull-request skill; verify CI is green
