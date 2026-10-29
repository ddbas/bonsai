import { describe, expect, it } from "vitest";
import {
  BonsaiError,
  currentWorktree,
  findSlotCheckedOutOnBranch,
  getInfo,
  getWorktree,
  listPool,
  lockSlot,
  managedRoot,
  prunePool,
  repoSlug,
  statusForPath,
  unlockSlot,
} from "../src/index.ts";
import { addSlots, makeWorld, POOL } from "./fakes.ts";

/** Make a slot look "available": clean and with HEAD merged into the default. */
function makeAvailable(world: ReturnType<typeof makeWorld>, slot: string) {
  world.git.ancestorOf.set(slot, true);
  world.git.refs.add("refs/heads/main");
}

describe("repoSlug / managedRoot", () => {
  it("derives the slug from the main repo root, lowercased", async () => {
    const w = makeWorld();
    w.git.commonDir = "/work/My_Repo/.git";
    expect(await repoSlug(w.deps)).toBe("my-repo");
  });

  it("resolves a relative common dir against the cwd", async () => {
    const w = makeWorld();
    w.git.commonDir = ".git";
    w.env.cwdValue = "/work/Thing";
    expect(await repoSlug(w.deps)).toBe("thing");
  });

  it("rejects bare repositories", async () => {
    const w = makeWorld();
    w.git.commonDir = ".";
    await expect(repoSlug(w.deps)).rejects.toThrow("bare repositories");
  });

  it("uses BONSAI_ROOT, else ~/.bonsai, else errors", () => {
    const w = makeWorld();
    expect(managedRoot(w.deps)).toBe("/pool");
    w.env.vars.delete("BONSAI_ROOT");
    expect(managedRoot(w.deps)).toBe("/home/me/.bonsai");
    w.env.home = undefined;
    expect(() => managedRoot(w.deps)).toThrow(BonsaiError);
  });
});

describe("getWorktree", () => {
  it("creates a new detached slot under the pool when none exist", async () => {
    const w = makeWorld();
    const slot = await getWorktree(w.deps, undefined);
    expect(slot).toBe(`${POOL}/a1b2c3d4`);
    expect(w.git.calls).toContain(
      `worktreeAdd ${POOL}/a1b2c3d4 ${w.git.headSha} -`,
    );
  });

  it.each(["new", "reset"] as const)(
    "passes %s branch mode to worktree add",
    async (kind) => {
      const w = makeWorld();
      await getWorktree(w.deps, { kind, name: "feat" });
      expect(w.git.calls).toContain(
        `worktreeAdd ${POOL}/a1b2c3d4 ${w.git.headSha} ${kind}:feat`,
      );
    },
  );

  it("reuses the first available slot and resets it to HEAD", async () => {
    const w = makeWorld();
    const [dirty, clean] = addSlots(w, [{}, {}]) as [string, string];
    w.git.statuses.set(dirty, {
      uncommitted: [],
      untracked: [{ code: "??", path: "x" }],
    });
    makeAvailable(w, clean);
    expect(await getWorktree(w.deps, undefined)).toBe(clean);
    expect(w.git.calls).toContain(`checkout ${clean} ${w.git.headSha} -`);
    expect(w.git.count("worktreeAdd")).toBe(0);
  });

  it("skips locked slots and slots with unmerged work", async () => {
    const w = makeWorld();
    const [locked, unmerged] = addSlots(w, [{ locked: true }, {}]) as [
      string,
      string,
    ];
    w.git.refs.add("refs/heads/main");
    w.git.ahead.set(unmerged, 1);
    const slot = await getWorktree(w.deps, undefined);
    expect(slot).not.toBe(locked);
    expect(slot).not.toBe(unmerged);
    expect(w.git.count("worktreeAdd")).toBe(1);
    // locked slot never triggers git status
    expect(w.git.calls.some((c) => c === `status ${locked}`)).toBe(false);
  });

  it("prunes stale registrations only when one exists", async () => {
    const w = makeWorld();
    addSlots(w, [{}]);
    makeAvailable(w, `${POOL}/slot0`);
    await getWorktree(w.deps, undefined);
    expect(w.git.count("worktreePrune")).toBe(0);

    const stale = makeWorld();
    addSlots(stale, [{}, {}]);
    stale.fs.paths.delete(`${POOL}/slot0`); // registered but missing on disk
    await getWorktree(stale.deps, undefined);
    expect(stale.git.count("worktreePrune")).toBe(1);
  });

  it("returns the existing slot already on a positional branch without touching it", async () => {
    const w = makeWorld();
    const [claimed] = addSlots(w, [{ branch: "shared" }, {}]) as [string];
    w.git.statuses.set(claimed, {
      uncommitted: [],
      untracked: [{ code: "??", path: "dirty" }],
    });
    expect(
      await getWorktree(w.deps, { kind: "existing", name: "shared" }),
    ).toBe(claimed);
    expect(w.git.count("checkout")).toBe(0);
    expect(w.git.count("worktreeAdd")).toBe(0);
  });

  it("does not short-circuit for -b / -B even if a slot has the branch", async () => {
    const w = makeWorld();
    addSlots(w, [{ branch: "feat" }]);
    await getWorktree(w.deps, { kind: "reset", name: "feat" });
    expect(w.git.count("worktreeAdd")).toBe(1);
  });

  it("copies configured bonsai.copy files, skipping missing ones", async () => {
    const w = makeWorld();
    w.git.config.set("bonsai.copy", [".env", ".env.missing", "config/a.json"]);
    w.fs.add("/work/myrepo/.env", "/work/myrepo/config/a.json");
    const slot = await getWorktree(w.deps, undefined);
    expect(w.fs.copies).toEqual([
      ["/work/myrepo/.env", `${slot}/.env`],
      ["/work/myrepo/config/a.json", `${slot}/config/a.json`],
    ]);
    expect(w.fs.paths.has(`${slot}/config`)).toBe(true); // parent created
  });

  it("copies into reused slots too", async () => {
    const w = makeWorld();
    const [slot] = addSlots(w, [{}]) as [string];
    makeAvailable(w, slot);
    w.git.config.set("bonsai.copy", [".env"]);
    w.fs.add("/work/myrepo/.env");
    await getWorktree(w.deps, undefined);
    expect(w.fs.copies).toEqual([["/work/myrepo/.env", `${slot}/.env`]]);
  });

  it("propagates git failures", async () => {
    const w = makeWorld();
    w.git.failures.set(
      "worktreeAdd",
      new BonsaiError("`git worktree add` failed: boom"),
    );
    await expect(getWorktree(w.deps, undefined)).rejects.toThrow("boom");
  });
});

describe("listPool", () => {
  it("is undefined when the pool does not exist yet", async () => {
    const w = makeWorld();
    expect(await listPool(w.deps)).toBeUndefined();
  });

  it("classifies each slot and keeps pool order", async () => {
    const w = makeWorld();
    const [locked, dirty, free] = addSlots(w, [
      { locked: true },
      { branch: "feat" },
      {},
    ]) as [string, string, string];
    w.git.statuses.set(dirty, {
      uncommitted: [{ code: " M", path: "a" }],
      untracked: [],
    });
    makeAvailable(w, free);
    const res = await listPool(w.deps);
    expect(res?.entries).toEqual([
      { path: locked, status: "locked", branch: undefined },
      { path: dirty, status: "in use", branch: "feat" },
      { path: free, status: "available", branch: undefined },
    ]);
  });

  it("short-circuits: locked never runs status, dirty never runs branch probes", async () => {
    const w = makeWorld();
    const [locked, dirty] = addSlots(w, [{ locked: true }, {}]) as [
      string,
      string,
    ];
    w.git.statuses.set(dirty, {
      uncommitted: [],
      untracked: [{ code: "??", path: "x" }],
    });
    await listPool(w.deps);
    expect(w.git.calls).not.toContain(`status ${locked}`);
    expect(w.git.count("isAncestor")).toBe(0);
    expect(w.git.count("symbolicRefShort")).toBe(0);
  });

  it("marks slots missing on disk as in use", async () => {
    const w = makeWorld();
    const [gone] = addSlots(w, [{}]) as [string];
    w.fs.paths.delete(gone);
    const res = await listPool(w.deps);
    expect(res?.entries[0]?.status).toBe("in use");
  });

  it("reports the slot containing the cwd as current", async () => {
    const w = makeWorld();
    const [a, b] = addSlots(w, [{}, {}]) as [string, string];
    w.fs.add(`${b}/sub`);
    w.env.cwdValue = `${b}/sub`;
    const res = await listPool(w.deps);
    expect(res?.current).toBe(b);
    expect(res?.current).not.toBe(a);
  });

  it("tolerates a failing current-slot lookup", async () => {
    const w = makeWorld();
    addSlots(w, [{}]);
    w.env.cwdValue = "/does/not/exist";
    const res = await listPool(w.deps);
    expect(res?.current).toBeUndefined();
  });
});

describe("statusForPath", () => {
  it("builds a full report for a dirty branch slot", async () => {
    const w = makeWorld();
    const [slot] = addSlots(w, [{ branch: "feat" }]) as [string];
    w.git.refs.add("refs/heads/main");
    w.git.ahead.set(slot, 3);
    w.git.statuses.set(slot, {
      uncommitted: [{ code: " M", path: "a.txt" }],
      untracked: [{ code: "??", path: "b.txt" }],
    });
    const report = await statusForPath(w.deps, slot);
    expect(report.status).toBe("in use");
    expect(report.branch).toBe("feat");
    expect(report.gitStatus.untracked).toHaveLength(1);
    expect(report.branchState).toContain("3 commits ahead of main");
  });

  it("does not evaluate branch state for locked slots", async () => {
    const w = makeWorld();
    const [slot] = addSlots(w, [{ locked: true, lockReason: "agent" }]) as [
      string,
    ];
    const report = await statusForPath(w.deps, slot);
    expect(report.status).toBe("locked");
    expect(report.lockReason).toBe("agent");
    expect(report.branchState).toBeUndefined();
    expect(w.git.count("isAncestor")).toBe(0);
  });

  it("defaults to the current slot and errors outside the pool", async () => {
    const w = makeWorld();
    const [slot] = addSlots(w, [{}]) as [string];
    makeAvailable(w, slot);
    w.env.cwdValue = slot;
    expect((await statusForPath(w.deps, undefined)).path).toBe(slot);

    w.env.cwdValue = "/work/myrepo";
    await expect(statusForPath(w.deps, undefined)).rejects.toThrow(
      "not inside a managed bonsai pool slot",
    );
  });

  it("rejects paths outside the pool or missing on disk", async () => {
    const w = makeWorld();
    addSlots(w, [{}]);
    await expect(statusForPath(w.deps, "/work/myrepo")).rejects.toThrow(
      "not a bonsai-managed pool slot",
    );
    await expect(statusForPath(w.deps, "/pool/nope")).rejects.toThrow(
      "does not exist",
    );
  });

  it("rejects unregistered directories inside the pool", async () => {
    const w = makeWorld();
    addSlots(w, [{}]);
    w.fs.add(`${POOL}/unregistered`);
    await expect(statusForPath(w.deps, `${POOL}/unregistered`)).rejects.toThrow(
      "not a registered git worktree",
    );
  });
});

describe("lockSlot / unlockSlot", () => {
  it("locks with a reason and logs", async () => {
    const w = makeWorld();
    const [slot] = addSlots(w, [{}]) as [string];
    expect(await lockSlot(w.deps, slot, "build")).toBe(slot);
    expect(w.git.calls).toContain(`worktreeLock ${slot} build`);
    expect(w.log.lines.some((l) => l.startsWith("info Locking slot"))).toBe(
      true,
    );
  });

  it("defaults to the current slot", async () => {
    const w = makeWorld();
    const [slot] = addSlots(w, [{}]) as [string];
    w.env.cwdValue = slot;
    expect(await unlockSlot(w.deps, undefined)).toBe(slot);
    expect(w.git.calls).toContain(`worktreeUnlock ${slot}`);
  });

  it("logs a warning and rethrows when git fails", async () => {
    const w = makeWorld();
    const [slot] = addSlots(w, [{}]) as [string];
    w.git.failures.set("worktreeLock", new BonsaiError("already locked"));
    await expect(lockSlot(w.deps, slot, undefined)).rejects.toThrow(
      "already locked",
    );
    expect(w.log.lines.some((l) => l.startsWith("warn Failed to lock"))).toBe(
      true,
    );
  });

  it("refuses paths outside the pool", async () => {
    const w = makeWorld();
    addSlots(w, [{}]);
    await expect(lockSlot(w.deps, "/work/myrepo", undefined)).rejects.toThrow(
      "not a bonsai-managed pool slot",
    );
  });
});

describe("prunePool", () => {
  it("is undefined without a pool directory", async () => {
    const w = makeWorld();
    expect(await prunePool(w.deps)).toBeUndefined();
  });

  it("never touches locked or in-use slots", async () => {
    const w = makeWorld();
    const [locked, dirty] = addSlots(w, [{ locked: true }, {}]) as [
      string,
      string,
    ];
    w.git.statuses.set(dirty, {
      uncommitted: [],
      untracked: [{ code: "??", path: "x" }],
    });
    const outcome = await prunePool(w.deps);
    expect(outcome?.pruned).toEqual([]);
    expect(outcome?.preserved).toBeUndefined();
    expect(w.fs.removed).toEqual([]);
    expect(w.fs.paths.has(locked) && w.fs.paths.has(dirty)).toBe(true);
    expect(w.git.count("worktreePrune")).toBe(1);
  });

  it("preserves exactly one available slot (the first) and deletes the rest", async () => {
    const w = makeWorld();
    const [a, b, c] = addSlots(w, [{}, {}, {}]) as [string, string, string];
    for (const s of [a, b, c]) makeAvailable(w, s);
    const outcome = await prunePool(w.deps);
    expect(outcome?.preserved?.path).toBe(a);
    expect(outcome?.pruned.map((p) => p.path)).toEqual([b, c]);
    expect(w.fs.removed).toEqual([b, c]);
    expect(w.git.count("checkout")).toBe(0); // already detached
    expect(w.git.count("worktreePrune")).toBe(1);
  });

  it("detaches the preserved slot's branch and reports pruned branches", async () => {
    const w = makeWorld();
    const [a, b] = addSlots(w, [{ branch: "keep" }, { branch: "drop" }]) as [
      string,
      string,
    ];
    for (const s of [a, b]) makeAvailable(w, s);
    const outcome = await prunePool(w.deps);
    expect(w.git.calls).toContain(`checkout ${a} ${w.git.headSha} -`);
    expect(outcome?.preserved).toEqual({ path: a, branch: "keep" });
    expect(outcome?.pruned).toEqual([{ path: b, branch: "drop" }]);
  });

  it("collects per-slot failures and still runs git worktree prune", async () => {
    const w = makeWorld();
    const [a, b, c] = addSlots(w, [{}, {}, {}]) as [string, string, string];
    for (const s of [a, b, c]) makeAvailable(w, s);
    w.fs.removeErrors.set(b, new Error("EACCES"));
    const outcome = await prunePool(w.deps);
    expect(outcome?.pruned.map((p) => p.path)).toEqual([c]);
    expect(outcome?.failures).toHaveLength(1);
    expect(outcome?.failures[0]?.path).toBe(b);
    expect(outcome?.failures[0]?.error).toContain("EACCES");
    expect(w.git.count("worktreePrune")).toBe(1);
  });

  it("records a preserve failure when detaching fails", async () => {
    const w = makeWorld();
    const [a] = addSlots(w, [{ branch: "keep" }]) as [string];
    makeAvailable(w, a);
    w.git.failures.set("checkout", new BonsaiError("checkout boom"));
    const outcome = await prunePool(w.deps);
    expect(outcome?.preserved).toBeUndefined();
    expect(outcome?.preserveFailure).toEqual({
      path: a,
      error: "checkout boom",
    });
    expect(w.fs.removed).toEqual([]);
  });
});

describe("getInfo", () => {
  const logs = {
    dir: () => "/home/me/.local/state/bonsai/logs" as string | undefined,
    currentFile: (d: string) => `${d}/bonsai.log.2026-01-02`,
  };

  it("tilde-abbreviates resolved paths and reports metadata", () => {
    const w = makeWorld();
    expect(
      getInfo(w.deps, logs, { version: "1.2.3", logLevel: "debug" }),
    ).toEqual({
      version: "1.2.3",
      logLevel: "debug",
      logDirectory: "~/.local/state/bonsai/logs",
      currentLogFile: "~/.local/state/bonsai/logs/bonsai.log.2026-01-02",
      managedRoot: "/pool",
    });
  });

  it("errors when the log directory cannot be resolved", () => {
    const w = makeWorld();
    expect(() =>
      getInfo(
        w.deps,
        { ...logs, dir: () => undefined },
        { version: "1", logLevel: "info" },
      ),
    ).toThrow("Failed to resolve log directory");
  });

  it("performs no filesystem writes", () => {
    const w = makeWorld();
    const before = new Set(w.fs.paths);
    getInfo(w.deps, logs, { version: "1", logLevel: "info" });
    expect(w.fs.paths).toEqual(before);
  });
});

describe("findSlotCheckedOutOnBranch", () => {
  it("is undefined for an absent pool or no match", async () => {
    const w = makeWorld();
    expect(await findSlotCheckedOutOnBranch(w.deps, POOL, "x")).toBeUndefined();
    addSlots(w, [{ branch: "other" }, {}]);
    expect(await findSlotCheckedOutOnBranch(w.deps, POOL, "x")).toBeUndefined();
  });

  it("matches a single slot, including a locked one", async () => {
    const w = makeWorld();
    const [, locked] = addSlots(w, [
      { branch: "a" },
      { branch: "b", locked: true },
    ]) as [string, string];
    expect(await findSlotCheckedOutOnBranch(w.deps, POOL, "b")).toBe(locked);
  });
});

describe("currentWorktree", () => {
  it("is undefined for an absent pool and for a cwd outside every slot", async () => {
    const w = makeWorld();
    expect(await currentWorktree(w.deps)).toBeUndefined();
    addSlots(w, [{}]);
    expect(await currentWorktree(w.deps)).toBeUndefined();
  });

  it("matches the slot that is an ancestor of the cwd", async () => {
    const w = makeWorld();
    const [slot] = addSlots(w, [{ branch: "feat" }]) as [string];
    w.fs.add(`${slot}/deep/dir`);
    w.env.cwdValue = `${slot}/deep/dir`;
    expect(await currentWorktree(w.deps)).toEqual({
      path: slot,
      branch: "feat",
    });
  });
});
