import type {
  BranchMode,
  Deps,
  Env,
  Fs,
  Git,
  GitStatusDetail,
  Logger,
  WorktreeEntry,
} from "../src/index.ts";

export const EMPTY_STATUS: GitStatusDetail = { uncommitted: [], untracked: [] };

/** In-memory `Git` double. Every public field can be tweaked per test. */
export class FakeGit implements Git {
  calls: string[] = [];
  headSha = "a".repeat(40);
  commonDir = "/work/myrepo/.git";
  worktrees: WorktreeEntry[] = [];
  statuses = new Map<string, GitStatusDetail>();
  config = new Map<string, string[]>();
  /** Refs that exist (full or short names, matched literally). */
  refs = new Set<string>();
  symbolic = new Map<string, string>();
  remotes = new Set<string>();
  /** `isAncestor(HEAD, d)` result per slot (default false). */
  ancestorOf = new Map<string, boolean>();
  ahead = new Map<string, number>();
  cherryOut: string | null = "";
  mergeTreeOut: string | null = null;
  trees = new Map<string, string>();
  failures = new Map<string, Error>();
  /** Called after a slot is created, so a fake fs can materialise it. */
  onWorktreeAdd: (slot: string) => void = () => {};
  /** Delay (ms) every status/branch probe takes, to simulate git cost. */
  probeDelayMs = 0;
  /** Delay (ms) `git worktree list` takes. */
  listDelayMs = 0;
  /** Bound on simultaneous simulated git processes (like the real adapter). */
  maxConcurrency = Number.POSITIVE_INFINITY;
  private active = 0;
  private readonly waiters: (() => void)[] = [];

  /** Simulate one git process: bounded concurrency + a fixed cost. */
  private async spend(ms: number): Promise<void> {
    if (ms <= 0) return;
    if (this.active >= this.maxConcurrency) {
      await new Promise<void>((r) => this.waiters.push(r));
    }
    this.active++;
    try {
      await new Promise((r) => setTimeout(r, ms));
    } finally {
      this.active--;
      this.waiters.shift()?.();
    }
  }

  private async tick(call: string): Promise<void> {
    this.calls.push(call);
    const failure = this.failures.get(call.split(" ")[0] ?? call);
    if (failure) throw failure;
    await this.spend(this.probeDelayMs);
  }

  count(prefix: string): number {
    return this.calls.filter((c) => c.startsWith(prefix)).length;
  }

  async revParseHeadAndCommonDir() {
    this.calls.push("revParseHeadAndCommonDir");
    return { headSha: this.headSha, commonDir: this.commonDir };
  }
  async revParseCommonDir() {
    this.calls.push("revParseCommonDir");
    return this.commonDir;
  }
  async resolveHead() {
    await this.tick("resolveHead");
    return this.headSha;
  }
  async listWorktrees() {
    this.calls.push("listWorktrees");
    await this.spend(this.listDelayMs);
    return this.worktrees.map((w) => ({ ...w }));
  }
  async status(dir: string) {
    await this.tick(`status ${dir}`);
    return this.statuses.get(dir) ?? EMPTY_STATUS;
  }
  async configGetAll(_dir: string, key: string) {
    this.calls.push(`configGetAll ${key}`);
    return this.config.get(key) ?? [];
  }
  async worktreePrune() {
    await this.tick("worktreePrune");
  }
  async worktreeLock(path: string, reason: string | undefined) {
    await this.tick(`worktreeLock ${path} ${reason ?? ""}`.trimEnd());
  }
  async worktreeUnlock(path: string) {
    await this.tick(`worktreeUnlock ${path}`);
  }
  async checkout(slot: string, head: string, b: BranchMode | undefined) {
    await this.tick(
      `checkout ${slot} ${head} ${b ? `${b.kind}:${b.name}` : "-"}`,
    );
  }
  async resetHardAndClean(slot: string) {
    await this.tick(`resetHardAndClean ${slot}`);
  }
  async worktreeAdd(slot: string, head: string, b: BranchMode | undefined) {
    await this.tick(
      `worktreeAdd ${slot} ${head} ${b ? `${b.kind}:${b.name}` : "-"}`,
    );
    this.onWorktreeAdd(slot);
  }

  async symbolicRefShort(_dir: string, ref: string) {
    await this.tick(`symbolicRefShort ${ref}`);
    return this.symbolic.get(ref) ?? null;
  }
  async refExists(_dir: string, ref: string) {
    await this.tick(`refExists ${ref}`);
    return this.refs.has(ref);
  }
  async remoteUrl(_dir: string, remote: string) {
    await this.tick(`remoteUrl ${remote}`);
    return this.remotes.has(remote) ? `/remote/${remote}` : null;
  }
  async isAncestor(dir: string, _a: string, _d: string) {
    await this.tick(`isAncestor ${dir}`);
    return this.ancestorOf.get(dir) ?? false;
  }
  async countCommits(dir: string, _range: string) {
    await this.tick(`countCommits ${dir}`);
    return this.ahead.get(dir) ?? 0;
  }
  async configValue(_dir: string, key: string) {
    await this.tick(`configValue ${key}`);
    return this.config.get(key)?.[0] ?? null;
  }
  async cherry(dir: string, _upstream: string) {
    await this.tick(`cherry ${dir}`);
    return this.cherryOut;
  }
  async mergeTree(dir: string, _a: string, _b: string) {
    await this.tick(`mergeTree ${dir}`);
    return this.mergeTreeOut;
  }
  async treeOf(_dir: string, rev: string) {
    await this.tick(`treeOf ${rev}`);
    return this.trees.get(rev) ?? null;
  }
}

/** In-memory filesystem: a set of existing paths plus a symlink map. */
export class FakeFs implements Fs {
  paths = new Set<string>();
  links = new Map<string, string>();
  copies: [string, string][] = [];
  removed: string[] = [];
  removeErrors = new Map<string, Error>();

  add(...paths: string[]): this {
    for (const p of paths) this.paths.add(p);
    return this;
  }
  async exists(p: string) {
    return this.paths.has(this.links.get(p) ?? p);
  }
  async realpath(p: string) {
    const target = this.links.get(p) ?? p;
    if (!this.paths.has(target)) throw new Error(`ENOENT: ${p}`);
    return target;
  }
  async mkdirp(p: string) {
    this.paths.add(p);
  }
  async copyFile(src: string, dest: string) {
    this.copies.push([src, dest]);
    this.paths.add(dest);
  }
  async removeDir(p: string) {
    const err = this.removeErrors.get(p);
    if (err) throw err;
    this.removed.push(p);
    this.paths.delete(p);
  }
}

export class FakeEnv implements Env {
  cwdValue = "/work/myrepo";
  vars = new Map<string, string>([["BONSAI_ROOT", "/pool"]]);
  home: string | undefined = "/home/me";
  nextIds = ["a1b2c3d4", "e5f60718", "99aabbcc", "0123abcd"];
  cwd() {
    return this.cwdValue;
  }
  get(name: string) {
    return this.vars.get(name);
  }
  homeDir() {
    return this.home;
  }
  randomSlotId() {
    return this.nextIds.shift() ?? "deadbeef";
  }
}

export class FakeLogger implements Logger {
  lines: string[] = [];
  debug(m: string) {
    this.lines.push(`debug ${m}`);
  }
  info(m: string) {
    this.lines.push(`info ${m}`);
  }
  warn(m: string) {
    this.lines.push(`warn ${m}`);
  }
  error(m: string) {
    this.lines.push(`error ${m}`);
  }
}

export function makeWorld() {
  const git = new FakeGit();
  const fs = new FakeFs().add("/", "/work", "/work/myrepo");
  const env = new FakeEnv();
  const log = new FakeLogger();
  git.onWorktreeAdd = (slot) => fs.add(slot);
  const deps: Deps = { git, fs, env, log };
  return { git, fs, env, log, deps };
}

/** Pool directory used by `makeWorld` (`$BONSAI_ROOT/<slug>`). */
export const POOL = "/pool/myrepo";

/** Register `n` pool slots (existing on disk) in the fake world. */
export function addSlots(
  world: ReturnType<typeof makeWorld>,
  specs: Partial<WorktreeEntry>[],
): string[] {
  world.fs.add("/pool", POOL);
  return specs.map((spec, i) => {
    const path = spec.path ?? `${POOL}/slot${i}`;
    world.fs.add(path);
    world.git.worktrees.push({
      path,
      locked: false,
      lockReason: undefined,
      branch: undefined,
      ...spec,
    });
    return path;
  });
}
