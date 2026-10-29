/**
 * Shared helpers for the e2e suite.
 *
 * Every test that touches git, the filesystem or home-directory behaviour MUST
 * use {@link GitEnv} rather than creating temp dirs + host git commands
 * directly. `GitEnv` backs each test with a Docker container (testcontainers)
 * that provides a fully isolated git environment: clean `gitconfig`, no host
 * hooks, no leaked `GIT_*` env vars, and a pinned git image.
 *
 * The binary under test is selected with `BS_BIN`. It defaults to the built
 * TypeScript bundle (`apps/bonsai/dist/cli.cjs`). Script paths (`.cjs`, `.js`,
 * `.mjs`) are run via `node`; anything else is executed directly (e.g. the
 * Rust binary `target/debug/bs`).
 */
import { type SpawnSyncOptions, spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  GenericContainer,
  type StartedTestContainer,
  Wait,
} from "testcontainers";
import { expect, onTestFinished } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));

// ── binary under test ─────────────────────────────────────────────────────────

/** Resolve `BS_BIN` (default: the built bundle) to a command + leading args. */
export function resolveBs(): { command: string; args: string[] } {
  const bin = resolve(
    process.env.BS_BIN ?? join(here, "../../../dist/cli.cjs"),
  );
  if (/\.(cjs|mjs|js)$/.test(bin)) {
    return { command: process.execPath, args: [bin] };
  }
  return { command: bin, args: [] };
}

export interface Out {
  status: number | null;
  stdout: string;
  stderr: string;
}

export interface RunOpts {
  cwd?: string;
  env?: Record<string, string | undefined>;
}

function run(command: string, args: string[], opts: SpawnSyncOptions): Out {
  const r = spawnSync(command, args, { encoding: "utf8", ...opts });
  if (r.error) throw r.error;
  return {
    status: r.status,
    stdout: String(r.stdout ?? ""),
    stderr: String(r.stderr ?? ""),
  };
}

// ── hello-world canary ────────────────────────────────────────────────────────

/** Start a lightweight `hello-world` container (Docker/testcontainers canary). */
export async function startGenericContainer(): Promise<StartedTestContainer> {
  return new GenericContainer("hello-world")
    .withWaitStrategy(Wait.forLogMessage("Hello from Docker!"))
    .start();
}

// ── GitEnv ────────────────────────────────────────────────────────────────────

const GIT_IMAGE = "alpine/git:latest";

/**
 * A fully isolated git test environment backed by a Docker container.
 *
 * - An `alpine/git` container is kept alive for the test (entrypoint `sh -c
 *   "tail -f /dev/null"`) so `exec` commands can be issued at any point.
 * - The repository lives in a host temp dir bind-mounted at `/workspace`, so
 *   both the container's `git` and the host `bs` operate on the same files.
 * - `BONSAI_ROOT` is a separate host temp dir.
 *
 * All git *setup* and *mutation* commands MUST go through {@link GitEnv.git}.
 */
export class GitEnv {
  private constructor(
    private readonly container: StartedTestContainer,
    private readonly repoTmp: string,
    private readonly bonsaiTmp: string,
    /** Canonicalised host path of the git repository (`/workspace`). */
    readonly repoPath: string,
    /** Canonicalised host path used as `BONSAI_ROOT`. */
    readonly bonsaiPath: string,
  ) {}

  /** Start the container and initialise a repository with one commit. */
  static async create(): Promise<GitEnv> {
    const repoTmp = mkdtempSync(join(tmpdir(), "bonsai-repo-"));
    const bonsaiTmp = mkdtempSync(join(tmpdir(), "bonsai-root-"));
    // Canonicalise so Docker Desktop's bind mounts resolve on macOS.
    const repoPath = realpathSync(repoTmp);
    const bonsaiPath = realpathSync(bonsaiTmp);

    const container = await new GenericContainer(GIT_IMAGE)
      .withEntrypoint(["sh"])
      .withCommand(["-c", "tail -f /dev/null"])
      .withWaitStrategy(Wait.forSuccessfulCommand("true"))
      .withBindMounts([{ source: repoPath, target: "/workspace" }])
      .start();

    const env = new GitEnv(container, repoTmp, bonsaiTmp, repoPath, bonsaiPath);
    onTestFinished(() => env.dispose());

    // Git 2.35.2+ refuses to operate in directories owned by another user;
    // trust /workspace globally BEFORE `git init`.
    await env.git(
      "config",
      "--global",
      "--add",
      "safe.directory",
      "/workspace",
    );
    await env.git("init");
    await env.git("config", "user.email", "test@example.com");
    await env.git("config", "user.name", "Test");
    await env.git("config", "commit.gpgsign", "false");

    writeFileSync(join(env.repoPath, "README.md"), "# test");
    await env.git("add", ".");
    await env.git("commit", "-m", "init");

    await env.fixPermissions();
    return env;
  }

  async dispose(): Promise<void> {
    await this.container.stop({ timeout: 1 }).catch(() => {});
    rmSync(this.repoTmp, { recursive: true, force: true });
    rmSync(this.bonsaiTmp, { recursive: true, force: true });
  }

  /** Run `git -C /workspace <args>` in the container; throws on failure. */
  async git(...args: string[]): Promise<void> {
    const cmd = ["git", "-C", "/workspace", ...args];
    const r = await this.container.exec(cmd);
    expect(
      r.exitCode,
      `git ${JSON.stringify(args)} exited with ${r.exitCode}:\n${r.output}`,
    ).toBe(0);
  }

  /**
   * Run `bs <args>` with `cwd` = the repo (or `opts.cwd`) and `BONSAI_ROOT` set.
   * Injects `safe.directory=*` via `GIT_CONFIG_*` so git subprocesses trust the
   * bind-mounted repo regardless of file ownership.
   */
  bs(args: string[] = [], opts: RunOpts = {}): Out {
    const { command, args: lead } = resolveBs();
    return run(command, [...lead, ...args], {
      cwd: opts.cwd ?? this.repoPath,
      env: cleanEnv({
        ...process.env,
        BONSAI_ROOT: this.bonsaiPath,
        GIT_CONFIG_COUNT: "1",
        GIT_CONFIG_KEY_0: "safe.directory",
        GIT_CONFIG_VALUE_0: "*",
        ...opts.env,
      }),
    });
  }

  /** Like {@link GitEnv.bs} but run from `dir` (e.g. a linked worktree). */
  bsFrom(dir: string, args: string[] = [], opts: RunOpts = {}): Out {
    return this.bs(args, { ...opts, cwd: dir });
  }

  /** Run `bs get`, assert success, return the slot path. */
  runGet(): string {
    const out = this.bs(["get"]);
    expect(
      out.status,
      `bs get failed\nstdout: ${out.stdout}\nstderr: ${out.stderr}`,
    ).toBe(0);
    return pathFromOutput(out.stdout);
  }

  /** Run `bs get <branch>`, assert success, return the slot path. */
  runGetBranch(branch: string): string {
    const out = this.bs(["get", branch]);
    expect(
      out.status,
      `bs get ${branch} failed\nstdout: ${out.stdout}\nstderr: ${out.stderr}`,
    ).toBe(0);
    return pathFromOutput(out.stdout);
  }

  /** Make `/workspace` world-accessible so the host user can write into it. */
  async fixPermissions(): Promise<void> {
    const r = await this.container.exec(["chmod", "-R", "a+rwX", "/workspace"]);
    expect(r.exitCode, `chmod failed: ${r.output}`).toBe(0);
  }

  /** HEAD SHA of the test repository. */
  headSha(): string {
    return worktreeHead(this.repoPath);
  }

  /** Write a file and create a new commit (via the container). */
  async makeCommit(filename: string, content: string): Promise<void> {
    writeFileSync(join(this.repoPath, filename), content);
    await this.git("add", ".");
    await this.git("commit", "-m", `add ${filename}`);
    await this.fixPermissions();
  }

  /**
   * Rename the current branch to `main` and give the repo a bare `origin`
   * (`/workspace/origin.git`) with `main` pushed and `origin/HEAD` set.
   */
  async addOrigin(): Promise<void> {
    await this.git("branch", "-M", "main");
    // Keep `git add .` in later helpers from sweeping the bare repo in.
    mkdirSync(join(this.repoPath, ".git/info"), { recursive: true });
    writeFileSync(join(this.repoPath, ".git/info/exclude"), "origin.git/\n");
    await this.git("init", "--bare", "-q", "origin.git");
    await this.git("remote", "add", "origin", "/workspace/origin.git");
    await this.git("push", "-u", "origin", "main");
    await this.git("remote", "set-head", "origin", "main");
    await this.fixPermissions();
  }

  /** Classification word (`available`/`in use`/`locked`) from `bs status`. */
  statusWord(slot: string): string {
    const out = this.bs(["status", slot]);
    expect(out.status, `bs status failed: ${out.stderr}`).toBe(0);
    const line = out.stdout.split("\n").find((l) => l.startsWith("status:"));
    if (line === undefined) throw new Error(`no status line in ${out.stdout}`);
    const word = ["available", "in use", "locked"].find((w) =>
      line.includes(w),
    );
    if (word === undefined) throw new Error(`unrecognized status line ${line}`);
    return word;
  }

  /** The single repo-slug directory under `bonsaiPath`. */
  slugDir(): string {
    const entries = readdirSync(this.bonsaiPath).map((e) => {
      const p = join(this.bonsaiPath, e);
      try {
        return realpathSync(p);
      } catch {
        return p;
      }
    });
    expect(
      entries.length,
      `expected exactly one slug dir under bonsaiPath, got: ${entries}`,
    ).toBe(1);
    return entries[0] as string;
  }

  /** All slot directories under the slug directory (sorted). */
  slots(): string[] {
    const slug = this.slugDir();
    return readdirSync(slug)
      .map((e) => join(slug, e))
      .sort();
  }
}

/** Drop undefined values so they are unset in the child environment. */
function cleanEnv(
  env: Record<string, string | undefined>,
): Record<string, string> {
  return Object.fromEntries(
    Object.entries(env).filter(
      (e): e is [string, string] => e[1] !== undefined,
    ),
  );
}

// ── Shared utilities ──────────────────────────────────────────────────────────

/**
 * Parse the worktree path from `bs get` stdout:
 * `🌳 /abs/path` or `🌳 /abs/path  (branch)`.
 */
export function pathFromOutput(raw: string): string {
  const s = raw.trim().replace(/^🌳/u, "").trim();
  const idx = s.indexOf("  (");
  return idx >= 0 ? s.slice(0, idx) : s;
}

/** Parse the branch from `bs get -b/-B` stdout (`null` when detached). */
export function branchFromOutput(raw: string): string | null {
  const s = raw.trim().replace(/^🌳/u, "").trim();
  const start = s.indexOf("  (");
  if (start < 0 || !s.endsWith(")")) return null;
  return s.slice(start + 3, -1);
}

/** HEAD SHA of the worktree at `path` (host git, hook env vars cleared). */
export function worktreeHead(path: string): string {
  return hostGit(path, ["rev-parse", "HEAD"]).stdout.trim();
}

/** Run `git` on the HOST in `dir` with hook-injected `GIT_*` vars cleared. */
export function hostGit(dir: string, args: string[]): Out {
  const env: Record<string, string | undefined> = { ...process.env };
  for (const v of [
    "GIT_DIR",
    "GIT_INDEX_FILE",
    "GIT_WORK_TREE",
    "GIT_PREFIX",
    "GIT_COMMON_DIR",
  ]) {
    delete env[v];
  }
  return run("git", args, { cwd: dir, env: cleanEnv(env) });
}

/**
 * A directory exposing only `git` (symlink to the host's real git) and
 * nothing else, for asserting a command needs no external tool besides git.
 * Cleaned up automatically when the test finishes.
 */
export function gitOnlyPathDir(): string {
  const which = run("which", ["git"], {});
  expect(which.status, "`which git` failed to locate git").toBe(0);
  const dir = mkdtempSync(join(tmpdir(), "bonsai-gitonly-"));
  symlinkSync(which.stdout.trim(), join(dir, "git"));
  onTestFinished(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

/** Run a host command (e.g. tmux) and return its result. */
export function hostRun(command: string, args: string[], opts: RunOpts = {}) {
  return run(command, args, {
    cwd: opts.cwd,
    env: opts.env ? cleanEnv({ ...process.env, ...opts.env }) : undefined,
  });
}

/** Assert a command exited 0, with stdout/stderr in the failure message. */
export function expectOk(out: Out, what: string): void {
  expect(
    out.status,
    `${what} failed\nstdout: ${out.stdout}\nstderr: ${out.stderr}`,
  ).toBe(0);
}

/** Assert a command exited non-zero. */
export function expectFail(out: Out, what: string): void {
  expect(out.status, `${what} should fail\nstdout: ${out.stdout}`).not.toBe(0);
}

/** Short branch of the worktree at `dir` (`HEAD` when detached). */
export function currentBranch(dir: string): string {
  return hostGit(dir, ["rev-parse", "--abbrev-ref", "HEAD"]).stdout.trim();
}

/** `~/rest` for paths under `$HOME`, otherwise the path unchanged. */
export function tildePath(path: string): string {
  const home = homedir();
  if (path === home) return "~/";
  if (path.startsWith(`${home}/`)) return `~/${path.slice(home.length + 1)}`;
  return path;
}

/** Strip ANSI CSI escape sequences (colour codes) from a string. */
export function stripAnsi(s: string): string {
  // biome-ignore lint/suspicious/noControlCharactersInRegex: matching ESC
  return s.replace(/\u001b\[[0-9;]*[A-Za-z]/g, "");
}
