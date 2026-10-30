/**
 * Ports: everything core needs from the outside world. Implemented by the
 * adapter packages (`@bonsai/git`, `@bonsai/tmux`, `@bonsai/logging`) and by
 * the app's composition root (`Fs`, `Env`).
 */
import type { BranchMode, GitStatusDetail, WorktreeEntry } from "./types.ts";

export interface Git {
  // ── repository / pool ───────────────────────────────────────────────────
  /** Raw `git rev-parse HEAD --git-common-dir` (path may be relative). */
  revParseHeadAndCommonDir(): Promise<{ headSha: string; commonDir: string }>;
  /** Raw `git rev-parse --git-common-dir` (path may be relative). */
  revParseCommonDir(): Promise<string>;
  /** Full SHA of HEAD. */
  resolveHead(): Promise<string>;
  /** Every registered worktree (paths as git reports them). */
  listWorktrees(): Promise<WorktreeEntry[]>;
  /** `git status --porcelain --untracked-files=all` for `dir`. */
  status(dir: string): Promise<GitStatusDetail>;
  /** `git config --get-all <key>` resolved from `dir`; `[]` when unset. */
  configGetAll(dir: string, key: string): Promise<string[]>;
  worktreePrune(): Promise<void>;
  worktreeLock(path: string, reason: string | undefined): Promise<void>;
  worktreeUnlock(path: string): Promise<void>;
  /** Reset an existing slot to `headSha`, optionally on a branch. */
  checkout(
    slot: string,
    headSha: string,
    branch: BranchMode | undefined,
  ): Promise<void>;
  /** Create a new slot at `slot`, optionally on a branch. */
  worktreeAdd(
    slot: string,
    headSha: string,
    branch: BranchMode | undefined,
  ): Promise<void>;

  // ── branch-state probes (all best-effort: failure ⇒ null/false) ─────────
  /** `git symbolic-ref -q --short <ref>`. */
  symbolicRefShort(dir: string, ref: string): Promise<string | null>;
  /** `git rev-parse --verify -q <ref>` succeeds. */
  refExists(dir: string, ref: string): Promise<boolean>;
  /** `git remote get-url <remote>`. */
  remoteUrl(dir: string, remote: string): Promise<string | null>;
  /** `git merge-base --is-ancestor <ancestor> <descendant>`. */
  isAncestor(
    dir: string,
    ancestor: string,
    descendant: string,
  ): Promise<boolean>;
  /** `git rev-list --count <range>`. */
  countCommits(dir: string, range: string): Promise<number | null>;
  /** `git config <key>`. */
  configValue(dir: string, key: string): Promise<string | null>;
  /** `git cherry <upstream> HEAD` stdout. */
  cherry(dir: string, upstream: string): Promise<string | null>;
  /** First line of `git merge-tree --write-tree <a> <b>` (the result tree). */
  mergeTree(dir: string, a: string, b: string): Promise<string | null>;
  /** `git rev-parse <rev>^{tree}`. */
  treeOf(dir: string, rev: string): Promise<string | null>;
}

export interface Tmux {
  /** Throws a `BonsaiError` when tmux is not available. */
  checkAvailable(): Promise<void>;
  hasSession(name: string): Promise<boolean>;
  /** Create a detached session rooted at `cwd`. */
  newSession(name: string, cwd: string): Promise<void>;
  /** Attach (or switch, when already inside tmux) the terminal. */
  attach(name: string): Promise<void>;
}

export interface Logger {
  debug(message: string): void;
  info(message: string): void;
  warn(message: string): void;
  error(message: string): void;
}

/** Where logs live; resolved without any I/O. */
export interface LogLocation {
  dir(): string | undefined;
  currentFile(dir: string): string;
}

export interface Fs {
  exists(path: string): Promise<boolean>;
  /** Canonicalise; rejects when the path does not exist. */
  realpath(path: string): Promise<string>;
  mkdirp(path: string): Promise<void>;
  copyFile(src: string, dest: string): Promise<void>;
  removeDir(path: string): Promise<void>;
}

export interface Env {
  cwd(): string;
  get(name: string): string | undefined;
  homeDir(): string | undefined;
  /** 8 lowercase hex characters (first group of a UUID v4). */
  randomSlotId(): string;
}

export interface Deps {
  git: Git;
  fs: Fs;
  env: Env;
  log: Logger;
}
