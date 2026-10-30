import { describe, expect, it } from "vitest";
import {
  type BranchState,
  branchState,
  isAvailable,
  LazyDefaultRef,
  reason,
  resolveDefaultRef,
} from "../src/index.ts";
import { FakeGit } from "./fakes.ts";

const D = "origin/main";

describe("availability", () => {
  it("only no-own-commits and merged-remote-gone are available", () => {
    expect(isAvailable({ kind: "no-own-commits" })).toBe(true);
    expect(isAvailable({ kind: "merged-remote-gone" })).toBe(true);
    for (const kind of [
      "unpushed",
      "remote-exists",
      "not-merged",
      "detached-with-commits",
    ] as const) {
      expect(isAvailable({ kind, n: 1 })).toBe(false);
    }
    expect(isAvailable({ kind: "default-unknown" })).toBe(false);
  });
});

describe("reasons", () => {
  it("unpushed mentions commits, upstream and the -u hint", () => {
    const r = reason({ kind: "unpushed", n: 3 }, D);
    expect(r).toContain("3 commits ahead of origin/main");
    expect(r).toContain("no upstream configured");
    expect(r).toContain("-u");
  });

  it("uses the singular for one commit", () => {
    const r = reason({ kind: "remote-exists", n: 1 }, D);
    expect(r).toContain("1 commit ahead");
    expect(r).toContain("remote branch still exists");
  });

  it("covers the remaining states", () => {
    expect(reason({ kind: "default-unknown" }, undefined)).toContain(
      "default branch could not be determined",
    );
    expect(reason({ kind: "no-own-commits" }, D)).toContain(
      "no commits of its own",
    );
    expect(reason({ kind: "merged-remote-gone" }, D)).toContain(
      "merged into origin/main, remote branch is gone",
    );
    expect(reason({ kind: "not-merged", n: 2 }, D)).toBe(
      "2 commits ahead of origin/main, not merged",
    );
    expect(reason({ kind: "detached-with-commits", n: 1 }, D)).toBe(
      "detached HEAD with 1 commit not on origin/main",
    );
  });
});

describe("resolveDefaultRef", () => {
  it("falls back to local main without a remote", async () => {
    const git = new FakeGit();
    git.refs.add("refs/heads/main");
    expect(await resolveDefaultRef(git, "/r")).toBe("main");
  });

  it("falls back to local master when main is absent", async () => {
    const git = new FakeGit();
    git.refs.add("refs/heads/master");
    expect(await resolveDefaultRef(git, "/r")).toBe("master");
  });

  it("is unresolvable when origin exists without refs", async () => {
    const git = new FakeGit();
    git.remotes.add("origin");
    git.refs.add("refs/heads/main"); // local main is ignored when origin exists
    expect(await resolveDefaultRef(git, "/r")).toBeUndefined();
  });

  it("prefers origin/HEAD, then origin/main, then origin/master", async () => {
    const git = new FakeGit();
    git.remotes.add("origin");
    git.refs.add("refs/remotes/origin/master");
    expect(await resolveDefaultRef(git, "/r")).toBe("origin/master");

    git.refs.add("refs/remotes/origin/main");
    expect(await resolveDefaultRef(git, "/r")).toBe("origin/main");

    git.symbolic.set("refs/remotes/origin/HEAD", "origin/develop");
    git.refs.add("origin/develop");
    expect(await resolveDefaultRef(git, "/r")).toBe("origin/develop");
  });

  it("ignores a dangling origin/HEAD", async () => {
    const git = new FakeGit();
    git.remotes.add("origin");
    git.symbolic.set("refs/remotes/origin/HEAD", "origin/gone");
    git.refs.add("refs/remotes/origin/main");
    expect(await resolveDefaultRef(git, "/r")).toBe("origin/main");
  });
});

describe("LazyDefaultRef", () => {
  it("resolves once and shares the result across concurrent callers", async () => {
    const git = new FakeGit();
    git.refs.add("refs/heads/main");
    const lazy = new LazyDefaultRef(git, "/r");
    expect(git.calls).toEqual([]); // nothing until first use
    const results = await Promise.all([lazy.get(), lazy.get(), lazy.get()]);
    expect(results).toEqual(["main", "main", "main"]);
    expect(git.count("symbolicRefShort")).toBe(1);
  });
});

/** Each README "When is a slot available?" case. */
describe("branchState", () => {
  const slot = "/pool/myrepo/s1";

  async function stateOf(
    setup: (git: FakeGit) => void,
    branch: string | undefined,
    d: string | null = D,
  ): Promise<BranchState> {
    const git = new FakeGit();
    setup(git);
    return branchState(git, slot, branch, d ?? undefined);
  }

  it("default unknown", async () => {
    expect(await stateOf(() => {}, "feat", null)).toEqual({
      kind: "default-unknown",
    });
  });

  it("fresh slot / fast-forward merged: HEAD is an ancestor of default", async () => {
    const s = await stateOf((g) => g.ancestorOf.set(slot, true), "feat");
    expect(s).toEqual({ kind: "no-own-commits" });
  });

  it("never pushed: no upstream configured", async () => {
    const s = await stateOf((g) => g.ahead.set(slot, 2), "feat");
    expect(s).toEqual({ kind: "unpushed", n: 2 });
  });

  it("upstream on '.' counts as unpushed", async () => {
    const s = await stateOf((g) => {
      g.ahead.set(slot, 1);
      g.config.set("branch.feat.remote", ["."]);
      g.config.set("branch.feat.merge", ["refs/heads/main"]);
    }, "feat");
    expect(s).toEqual({ kind: "unpushed", n: 1 });
  });

  it("detached HEAD with its own commits", async () => {
    const s = await stateOf((g) => g.ahead.set(slot, 4), undefined);
    expect(s).toEqual({ kind: "detached-with-commits", n: 4 });
  });

  it("merged but remote branch still present is in use", async () => {
    const s = await stateOf((g) => {
      g.ahead.set(slot, 1);
      g.config.set("branch.feat.remote", ["origin"]);
      g.config.set("branch.feat.merge", ["refs/heads/feat"]);
      g.refs.add("refs/remotes/origin/feat");
    }, "feat");
    expect(s).toEqual({ kind: "remote-exists", n: 1 });
  });

  function pushedThenRemoteGone(g: FakeGit): void {
    g.ahead.set(slot, 1);
    g.config.set("branch.feat.remote", ["origin"]);
    g.config.set("branch.feat.merge", ["refs/heads/feat"]);
  }

  it("rebase merged (cherry has no '+' lines), remote gone is available", async () => {
    const s = await stateOf((g) => {
      pushedThenRemoteGone(g);
      g.cherryOut = "- abc123";
    }, "feat");
    expect(s).toEqual({ kind: "merged-remote-gone" });
  });

  it("squash merged (merge-tree result equals default tree), remote gone is available", async () => {
    const s = await stateOf((g) => {
      pushedThenRemoteGone(g);
      g.cherryOut = "+ abc123";
      g.mergeTreeOut = "tree1";
      g.trees.set(D, "tree1");
    }, "feat");
    expect(s).toEqual({ kind: "merged-remote-gone" });
  });

  it("remote gone without a merge is not merged", async () => {
    const s = await stateOf((g) => {
      pushedThenRemoteGone(g);
      g.cherryOut = "+ abc123";
      g.mergeTreeOut = "tree-other";
      g.trees.set(D, "tree1");
    }, "feat");
    expect(s).toEqual({ kind: "not-merged", n: 1 });
  });

  it("a failing merge-tree (conflicts) counts as not merged", async () => {
    const s = await stateOf((g) => {
      pushedThenRemoteGone(g);
      g.cherryOut = null;
      g.mergeTreeOut = null;
    }, "feat");
    expect(s).toEqual({ kind: "not-merged", n: 1 });
  });

  it("cheapest check first: ancestry short-circuits all other probes", async () => {
    const git = new FakeGit();
    git.ancestorOf.set(slot, true);
    await branchState(git, slot, "feat", D);
    expect(git.calls).toEqual([`isAncestor ${slot}`]);
  });
});
