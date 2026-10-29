import { describe, expect, it } from "vitest";
import { parseWorktreeList } from "../src/index.ts";

describe("parseWorktreeList", () => {
  it("parses branch, detached, locked and locked-with-reason entries", () => {
    const text = [
      "worktree /repo",
      "HEAD aaaa",
      "branch refs/heads/main",
      "",
      "worktree /pool/one",
      "HEAD bbbb",
      "detached",
      "",
      "worktree /pool/two",
      "HEAD cccc",
      "branch refs/heads/feature/nested",
      "locked",
      "",
      "worktree /pool/three",
      "HEAD dddd",
      "detached",
      "locked reserved for agent build",
      "",
    ].join("\n");

    expect(parseWorktreeList(text)).toEqual([
      { path: "/repo", locked: false, lockReason: undefined, branch: "main" },
      {
        path: "/pool/one",
        locked: false,
        lockReason: undefined,
        branch: undefined,
      },
      {
        path: "/pool/two",
        locked: true,
        lockReason: undefined,
        branch: "feature/nested",
      },
      {
        path: "/pool/three",
        locked: true,
        lockReason: "reserved for agent build",
        branch: undefined,
      },
    ]);
  });

  it("does not leak branch/lock state between entries", () => {
    const text =
      "worktree /a\nbranch refs/heads/x\nlocked why\n\nworktree /b\ndetached\n";
    const [a, b] = parseWorktreeList(text);
    expect(a?.branch).toBe("x");
    expect(b).toEqual({
      path: "/b",
      locked: false,
      lockReason: undefined,
      branch: undefined,
    });
  });

  it("returns [] for empty output", () => {
    expect(parseWorktreeList("")).toEqual([]);
  });

  it("keeps a non-refs/heads branch ref verbatim", () => {
    expect(
      parseWorktreeList("worktree /a\nbranch refs/other/x\n")[0]?.branch,
    ).toBe("refs/other/x");
  });
});
