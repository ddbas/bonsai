import { describe, expect, it } from "vitest";
import {
  classify,
  parseLockedLine,
  parseStatusPorcelain,
  path,
  slugify,
} from "../src/index.ts";

describe("classify priority: locked > in use > available", () => {
  it("locked beats everything", () => {
    expect(classify(true, false, false)).toBe("locked");
    expect(classify(true, true, false)).toBe("locked");
    expect(classify(true, false, true)).toBe("locked");
    expect(classify(true, true, true)).toBe("locked");
  });

  it("dirty or unmerged work is in use", () => {
    expect(classify(false, true, false)).toBe("in use");
    expect(classify(false, false, true)).toBe("in use");
    expect(classify(false, true, true)).toBe("in use");
  });

  it("clean, unlocked, merged is available", () => {
    expect(classify(false, false, false)).toBe("available");
  });
});

describe("slugify", () => {
  it("lower-cases and replaces non-alphanumerics with dashes", () => {
    expect(slugify("My Repo_2.0")).toBe("my-repo-2-0");
    expect(slugify("bonsai")).toBe("bonsai");
    expect(slugify("ünï")).toBe("-n-");
  });
});

describe("parseLockedLine", () => {
  it("handles bare, reasoned and non-locked lines", () => {
    expect(parseLockedLine("locked")).toEqual([true, undefined]);
    expect(parseLockedLine("locked   ")).toEqual([true, undefined]);
    expect(parseLockedLine("locked reserved for agent")).toEqual([
      true,
      "reserved for agent",
    ]);
    expect(parseLockedLine("detached")).toEqual([false, undefined]);
  });
});

describe("parseStatusPorcelain", () => {
  it("splits uncommitted from untracked and keeps rename arrows", () => {
    const detail = parseStatusPorcelain(
      " M a.txt\n?? new.txt\nA  b.txt\nR  old -> new\n\n",
    );
    expect(detail.uncommitted).toEqual([
      { code: " M", path: "a.txt" },
      { code: "A ", path: "b.txt" },
      { code: "R ", path: "old -> new" },
    ]);
    expect(detail.untracked).toEqual([{ code: "??", path: "new.txt" }]);
  });

  it("returns empty lists for empty output", () => {
    expect(parseStatusPorcelain("")).toEqual({
      uncommitted: [],
      untracked: [],
    });
  });
});

describe("tildePath", () => {
  it("replaces the home prefix with ~", () => {
    expect(path.tildePath("/home/me/some/dir", "/home/me")).toBe("~/some/dir");
  });

  it("maps exactly home to ~/", () => {
    expect(path.tildePath("/home/me", "/home/me")).toBe("~/");
  });

  it("leaves paths outside home (or a sibling prefix) unchanged", () => {
    expect(path.tildePath("/tmp/some/path", "/home/me")).toBe("/tmp/some/path");
    expect(path.tildePath("/home/meow/x", "/home/me")).toBe("/home/meow/x");
    expect(path.tildePath("/x", undefined)).toBe("/x");
  });
});

describe("path helpers", () => {
  it("resolve normalises relative git common dirs", () => {
    expect(path.resolve("/work/repo", ".git")).toBe("/work/repo/.git");
    expect(path.resolve("/work/repo/sub", "../.git")).toBe("/work/repo/.git");
    expect(path.resolve("/x", "/abs/.git")).toBe("/abs/.git");
  });

  it("dirname/basename", () => {
    expect(path.dirname("/work/repo/.git")).toBe("/work/repo");
    expect(path.basename("/work/repo")).toBe("repo");
    expect(path.basename("/")).toBeUndefined();
    expect(path.dirname("/")).toBeUndefined();
  });

  it("startsWith is component-wise", () => {
    expect(path.startsWith("/a/b/c", "/a/b")).toBe(true);
    expect(path.startsWith("/a/bc", "/a/b")).toBe(false);
    expect(path.startsWith("/a/b", "/a/b")).toBe(true);
  });
});
