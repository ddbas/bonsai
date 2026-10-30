import {
  BonsaiError,
  type BranchMode,
  type Git,
  parseStatusPorcelain,
} from "@bonsai/core";
import { parseWorktreeList } from "./parse.ts";
import { createExecRunner, type Runner, type RunResult } from "./runner.ts";

export interface GitOptions {
  /** Maximum concurrent git processes (default 16). */
  maxConcurrency?: number;
  /** Override the process runner (tests). */
  runner?: Runner;
}

function branchFlag(mode: BranchMode): string[] {
  switch (mode.kind) {
    case "new":
      return ["-b", mode.name];
    case "reset":
      return ["-B", mode.name];
    case "existing":
      return [mode.name];
  }
}

/** Git adapter: implements the core `Git` port over the `git` CLI. */
export function createGit(options: GitOptions = {}): Git {
  const run = options.runner ?? createExecRunner(options.maxConcurrency ?? 16);

  /** Run git; throw `BonsaiError("`git <what>` failed: <stderr>")` on failure. */
  async function must(
    args: string[],
    what: string,
    suffix = "",
  ): Promise<RunResult> {
    const r = await run(args);
    if (r.code !== 0) {
      throw new BonsaiError(`\`${what}\` failed${suffix}: ${r.stderr.trim()}`);
    }
    return r;
  }

  /** Run `git -C <dir> …`; trimmed stdout on success, `null` otherwise. */
  async function ok(dir: string, args: string[]): Promise<string | null> {
    try {
      const r = await run(["-C", dir, ...args]);
      return r.code === 0 ? r.stdout.trim() : null;
    } catch {
      return null;
    }
  }

  return {
    async revParseHeadAndCommonDir() {
      const r = await must(
        ["rev-parse", "HEAD", "--git-common-dir"],
        "git rev-parse HEAD --git-common-dir",
      );
      const [head, commonDir] = r.stdout.split("\n").map((l) => l.trim());
      if (head === undefined || head === "") {
        throw new BonsaiError("`git rev-parse` produced no HEAD output");
      }
      if (commonDir === undefined || commonDir === "") {
        throw new BonsaiError(
          "`git rev-parse` produced no --git-common-dir output",
        );
      }
      return { headSha: head, commonDir };
    },

    async revParseCommonDir() {
      const r = await must(
        ["rev-parse", "--git-common-dir"],
        "git rev-parse --git-common-dir",
      );
      return r.stdout.trim();
    },

    async resolveHead() {
      const r = await must(["rev-parse", "HEAD"], "git rev-parse HEAD");
      return r.stdout.trim();
    },

    async listWorktrees() {
      const r = await must(
        ["worktree", "list", "--porcelain"],
        "git worktree list",
      );
      return parseWorktreeList(r.stdout);
    },

    async status(dir) {
      const r = await must(
        ["-C", dir, "status", "--porcelain", "--untracked-files=all"],
        "git status --porcelain",
        ` for ${dir}`,
      );
      return parseStatusPorcelain(r.stdout);
    },

    async configGetAll(dir, key) {
      const r = await run(["-C", dir, "config", "--get-all", key]);
      if (r.code !== 0) {
        // Exit 1 with empty stderr: key not set anywhere — not an error.
        if (r.stderr.trim() === "") return [];
        throw new BonsaiError(
          `\`git config --get-all ${key}\` failed: ${r.stderr.trim()}`,
        );
      }
      return r.stdout.split("\n").filter((l) => l !== "");
    },

    async worktreePrune() {
      await must(["worktree", "prune"], "git worktree prune");
    },

    async worktreeLock(path, reason) {
      const args =
        reason === undefined
          ? ["worktree", "lock", path]
          : ["worktree", "lock", "--reason", reason, path];
      await must(args, "git worktree lock");
    },

    async worktreeUnlock(path) {
      await must(["worktree", "unlock", path], "git worktree unlock");
    },

    async checkout(slot, headSha, branch) {
      const args =
        branch === undefined
          ? ["-C", slot, "checkout", "--detach", headSha]
          : branch.kind === "existing"
            ? ["-C", slot, "checkout", branch.name]
            : ["-C", slot, "checkout", ...branchFlag(branch), headSha];
      await must(args, "git checkout");
    },

    async resetHardAndClean(slot) {
      await must(["-C", slot, "reset", "--hard"], "git reset --hard");
      await must(["-C", slot, "clean", "-fd"], "git clean");
    },

    async worktreeAdd(slot, headSha, branch) {
      const args =
        branch === undefined
          ? ["worktree", "add", "--detach", slot, headSha]
          : branch.kind === "existing"
            ? ["worktree", "add", slot, branch.name]
            : ["worktree", "add", ...branchFlag(branch), slot, headSha];
      await must(args, "git worktree add");
    },

    // ── branch-state probes ──────────────────────────────────────────────

    symbolicRefShort: (dir, ref) =>
      ok(dir, ["symbolic-ref", "-q", "--short", ref]),

    async refExists(dir, ref) {
      return (await ok(dir, ["rev-parse", "--verify", "-q", ref])) !== null;
    },

    remoteUrl: (dir, remote) => ok(dir, ["remote", "get-url", remote]),

    async isAncestor(dir, ancestor, descendant) {
      return (
        (await ok(dir, [
          "merge-base",
          "--is-ancestor",
          ancestor,
          descendant,
        ])) !== null
      );
    },

    async countCommits(dir, range) {
      const out = await ok(dir, ["rev-list", "--count", range]);
      if (out === null) return null;
      const n = Number.parseInt(out, 10);
      return Number.isNaN(n) ? null : n;
    },

    configValue: (dir, key) => ok(dir, ["config", key]),

    cherry: (dir, upstream) => ok(dir, ["cherry", upstream, "HEAD"]),

    async mergeTree(dir, a, b) {
      const out = await ok(dir, ["merge-tree", "--write-tree", a, b]);
      if (out === null || out === "") return null;
      return out.split("\n")[0] ?? null;
    },

    treeOf: (dir, rev) => ok(dir, ["rev-parse", `${rev}^{tree}`]),
  };
}
