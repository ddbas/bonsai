import { execFileSync } from "node:child_process";
import { mkdtempSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BonsaiError } from "@bonsai/core";
import { afterEach, describe, expect, it } from "vitest";
import {
  cleanGitEnv,
  createExecRunner,
  createGit,
  HOOK_ENV_VARS,
  type Runner,
  type RunResult,
  Semaphore,
} from "../src/index.ts";

/** A runner that records argv and replies from a queue of canned results. */
function fakeRunner(...replies: Partial<RunResult>[]) {
  const calls: string[][] = [];
  const runner: Runner = async (args) => {
    calls.push(args);
    return { code: 0, stdout: "", stderr: "", ...(replies.shift() ?? {}) };
  };
  return { calls, runner };
}

describe("command construction", () => {
  it("checkout: detached, new, reset, existing", async () => {
    const { calls, runner } = fakeRunner();
    const git = createGit({ runner });
    await git.checkout("/s", "abc", undefined);
    await git.checkout("/s", "abc", { kind: "new", name: "b" });
    await git.checkout("/s", "abc", { kind: "reset", name: "b" });
    await git.checkout("/s", "abc", { kind: "existing", name: "b" });
    expect(calls).toEqual([
      ["-C", "/s", "checkout", "--detach", "abc"],
      ["-C", "/s", "checkout", "-b", "b", "abc"],
      ["-C", "/s", "checkout", "-B", "b", "abc"],
      ["-C", "/s", "checkout", "b"],
    ]);
  });

  it("worktree add: detached, new, reset, existing", async () => {
    const { calls, runner } = fakeRunner();
    const git = createGit({ runner });
    await git.worktreeAdd("/s", "abc", undefined);
    await git.worktreeAdd("/s", "abc", { kind: "new", name: "b" });
    await git.worktreeAdd("/s", "abc", { kind: "reset", name: "b" });
    await git.worktreeAdd("/s", "abc", { kind: "existing", name: "b" });
    expect(calls).toEqual([
      ["worktree", "add", "--detach", "/s", "abc"],
      ["worktree", "add", "-b", "b", "/s", "abc"],
      ["worktree", "add", "-B", "b", "/s", "abc"],
      ["worktree", "add", "/s", "b"],
    ]);
  });

  it("lock forwards the reason verbatim", async () => {
    const { calls, runner } = fakeRunner();
    const git = createGit({ runner });
    await git.worktreeLock("/s", undefined);
    await git.worktreeLock("/s", "a b  c");
    await git.worktreeUnlock("/s");
    expect(calls).toEqual([
      ["worktree", "lock", "/s"],
      ["worktree", "lock", "--reason", "a b  c", "/s"],
      ["worktree", "unlock", "/s"],
    ]);
  });

  it("status lists untracked files individually", async () => {
    const { calls, runner } = fakeRunner({ stdout: " M a\n?? dir/b\n" });
    const detail = await createGit({ runner }).status("/s");
    expect(calls[0]).toEqual([
      "-C",
      "/s",
      "status",
      "--porcelain",
      "--untracked-files=all",
    ]);
    expect(detail.uncommitted).toEqual([{ code: " M", path: "a" }]);
    expect(detail.untracked).toEqual([{ code: "??", path: "dir/b" }]);
  });

  it("rev-parse HEAD --git-common-dir returns both values", async () => {
    const { runner } = fakeRunner({ stdout: "abc123\n.git\n" });
    expect(await createGit({ runner }).revParseHeadAndCommonDir()).toEqual({
      headSha: "abc123",
      commonDir: ".git",
    });
  });
});

describe("error mapping", () => {
  it("surfaces trimmed stderr in a BonsaiError", async () => {
    const { runner } = fakeRunner({ code: 128, stderr: "fatal: nope\n" });
    const err = await createGit({ runner })
      .worktreeAdd("/s", "abc", undefined)
      .catch((e) => e);
    expect(err).toBeInstanceOf(BonsaiError);
    expect(err.message).toBe("`git worktree add` failed: fatal: nope");
  });

  it("status names the directory", async () => {
    const { runner } = fakeRunner({ code: 128, stderr: "bad" });
    await expect(createGit({ runner }).status("/slot")).rejects.toThrow(
      "`git status --porcelain` failed for /slot: bad",
    );
  });

  it("configGetAll: unset key (exit 1, no stderr) is empty; real failures throw", async () => {
    const unset = fakeRunner({ code: 1 });
    expect(
      await createGit({ runner: unset.runner }).configGetAll(
        "/o",
        "bonsai.copy",
      ),
    ).toEqual([]);

    const some = fakeRunner({ stdout: ".env\n\nconfig/a.json\n" });
    expect(
      await createGit({ runner: some.runner }).configGetAll(
        "/o",
        "bonsai.copy",
      ),
    ).toEqual([".env", "config/a.json"]);

    const broken = fakeRunner({ code: 128, stderr: "fatal: bad config" });
    await expect(
      createGit({ runner: broken.runner }).configGetAll("/o", "bonsai.copy"),
    ).rejects.toThrow("fatal: bad config");
  });

  it("probes swallow failures", async () => {
    const failing: Runner = async () => {
      throw new Error("spawn failed");
    };
    const git = createGit({ runner: failing });
    expect(await git.refExists("/d", "x")).toBe(false);
    expect(await git.symbolicRefShort("/d", "x")).toBeNull();
    expect(await git.isAncestor("/d", "a", "b")).toBe(false);
    expect(await git.countCommits("/d", "a..b")).toBeNull();
  });

  it("probe parsing: counts, merge-tree first line, tree ids", async () => {
    const git1 = createGit({ runner: fakeRunner({ stdout: "7\n" }).runner });
    expect(await git1.countCommits("/d", "main..HEAD")).toBe(7);
    const git2 = createGit({
      runner: fakeRunner({ stdout: "treeid\nextra\n" }).runner,
    });
    expect(await git2.mergeTree("/d", "a", "b")).toBe("treeid");
    const git3 = createGit({ runner: fakeRunner({ stdout: "" }).runner });
    expect(await git3.mergeTree("/d", "a", "b")).toBeNull();
    const git4 = createGit({ runner: fakeRunner({ code: 1 }).runner });
    expect(await git4.mergeTree("/d", "a", "b")).toBeNull();
  });
});

describe("environment handling", () => {
  it("strips hook-injected GIT_* variables but keeps the rest", () => {
    const env: NodeJS.ProcessEnv = { PATH: "/bin", GIT_AUTHOR_NAME: "x" };
    for (const v of HOOK_ENV_VARS) env[v] = "set";
    const cleaned = cleanGitEnv(env);
    for (const v of HOOK_ENV_VARS) expect(cleaned[v]).toBeUndefined();
    expect(cleaned.PATH).toBe("/bin");
    expect(cleaned.GIT_AUTHOR_NAME).toBe("x");
  });
});

describe("Semaphore", () => {
  it("never runs more than `max` tasks at once", async () => {
    const gate = new Semaphore(3);
    let active = 0;
    let peak = 0;
    await Promise.all(
      Array.from({ length: 20 }, () =>
        gate.run(async () => {
          active++;
          peak = Math.max(peak, active);
          await new Promise((r) => setTimeout(r, 2));
          active--;
        }),
      ),
    );
    expect(peak).toBe(3);
  });

  it("releases the slot when a task throws", async () => {
    const gate = new Semaphore(1);
    await expect(
      gate.run(async () => Promise.reject(new Error("x"))),
    ).rejects.toThrow("x");
    expect(await gate.run(async () => "ok")).toBe("ok");
  });
});

describe("real git (host)", () => {
  const dirs: string[] = [];
  afterEach(() => {
    for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
  });

  function tempRepo(): string {
    const dir = realpathSync(mkdtempSync(join(tmpdir(), "bonsai-git-")));
    dirs.push(dir);
    const env = cleanGitEnv(process.env);
    const run = (...args: string[]) =>
      execFileSync("git", ["-C", dir, ...args], { env });
    run("init", "-q", "-b", "main");
    run("config", "user.email", "t@e.com");
    run("config", "user.name", "T");
    run("config", "commit.gpgsign", "false");
    writeFileSync(join(dir, "f"), "x");
    run("add", ".");
    run("commit", "-q", "-m", "init");
    return dir;
  }

  it("ignores hook-injected GIT_DIR and reports statuses", async () => {
    const dir = tempRepo();
    writeFileSync(join(dir, "new.txt"), "n");
    const previous = process.env.GIT_DIR;
    process.env.GIT_DIR = join(tmpdir(), "definitely-not-a-repo");
    try {
      const git = createGit({ runner: createExecRunner(2) });
      const detail = await git.status(dir);
      expect(detail.untracked).toEqual([{ code: "??", path: "new.txt" }]);
      expect(await git.refExists(dir, "refs/heads/main")).toBe(true);
      expect(await git.isAncestor(dir, "HEAD", "main")).toBe(true);
    } finally {
      if (previous === undefined) delete process.env.GIT_DIR;
      else process.env.GIT_DIR = previous;
    }
  });

  it("reports a spawn failure as a BonsaiError", async () => {
    const original = process.env.PATH;
    process.env.PATH = "/nonexistent";
    try {
      const git = createGit({ runner: createExecRunner(1) });
      await expect(git.resolveHead()).rejects.toThrow("failed to spawn");
    } finally {
      process.env.PATH = original;
    }
  });
});
