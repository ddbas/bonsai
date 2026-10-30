import type { Deps, Tmux, WorktreeEntry } from "@bonsai/core";
import { describe, expect, it } from "vitest";
import { stripAnsi } from "../tests/e2e/support/ansi.ts";
import {
  formatCurrentPath,
  formatListRow,
  formatStatusReport,
} from "./format.ts";
import type { Io } from "./io.ts";
import { buildProgram } from "./program.ts";
import type { Runtime } from "./runtime.ts";

/** A runtime backed by a scripted in-memory git/fs; records tmux calls. */
function harness(entries: WorktreeEntry[] = []) {
  const out: string[] = [];
  const err: string[] = [];
  const io: Io = { out: (s) => out.push(s), err: (s) => err.push(s) };
  const tmuxCalls: string[] = [];
  const logs: string[] = [];
  let tmuxMissing = false;
  let levelSeen: string | undefined;

  const existing = new Set<string>(["/pool", "/pool/repo", "/work/repo"]);
  for (const e of entries) existing.add(e.path);
  let added: string | undefined;

  const deps = {
    git: {
      revParseHeadAndCommonDir: async () => ({
        headSha: "abc",
        commonDir: "/work/repo/.git",
      }),
      revParseCommonDir: async () => "/work/repo/.git",
      listWorktrees: async () => entries,
      status: async () => ({ uncommitted: [], untracked: [] }),
      configGetAll: async () => [],
      worktreePrune: async () => {},
      worktreeAdd: async (slot: string) => {
        added = slot;
        existing.add(slot);
      },
      checkout: async () => {},
      worktreeLock: async () => {},
      worktreeUnlock: async () => {},
      resolveHead: async () => "abc",
      symbolicRefShort: async () => null,
      refExists: async (_d: string, r: string) => r === "refs/heads/main",
      remoteUrl: async () => null,
      isAncestor: async () => true,
      countCommits: async () => 0,
      configValue: async () => null,
      cherry: async () => null,
      mergeTree: async () => null,
      treeOf: async () => null,
    },
    fs: {
      exists: async (p: string) => existing.has(p),
      realpath: async (p: string) => {
        if (!existing.has(p)) throw new Error("ENOENT");
        return p;
      },
      mkdirp: async (p: string) => void existing.add(p),
      copyFile: async () => {},
      removeDir: async () => {},
    },
    env: {
      cwd: () => "/work/repo",
      get: (n: string) => (n === "BONSAI_ROOT" ? "/pool" : undefined),
      homeDir: () => "/home/me",
      randomSlotId: () => "cafebabe",
    },
    log: {
      debug: (m: string) => logs.push(m),
      info: (m: string) => logs.push(m),
      warn: (m: string) => logs.push(m),
      error: (m: string) => logs.push(m),
    },
  } as unknown as Deps;

  const tmux: Tmux = {
    checkAvailable: async () => {
      if (tmuxMissing) throw new Error("tmux was not found on PATH");
    },
    hasSession: async () => false,
    newSession: async (n, c) => void tmuxCalls.push(`new ${n} ${c}`),
    attach: async (n) => void tmuxCalls.push(`attach ${n}`),
  };

  const runtime: Runtime = {
    deps,
    tmux,
    logs: {
      dir: () => "/home/me/.local/state/bonsai/logs",
      currentFile: (d) => `${d}/bonsai.log.2026-01-01`,
    },
    close: () => {},
  };

  const bs = buildProgram({
    version: "9.9.9",
    io,
    exitOverride: true,
    runtime: ({ logLevel }) => {
      levelSeen = logLevel;
      return runtime;
    },
  });

  async function run(...args: string[]): Promise<{ code: number }> {
    try {
      await bs.program.parseAsync(args, { from: "user" });
      return { code: bs.exitCode() };
    } catch (e) {
      const code = (e as { exitCode?: number }).exitCode;
      if (code === undefined) throw e;
      return { code };
    }
  }

  return {
    run,
    stdout: () => stripAnsi(out.join("")),
    stderr: () => stripAnsi(err.join("")),
    tmuxCalls,
    logs,
    added: () => added,
    level: () => levelSeen,
    setTmuxMissing: () => {
      tmuxMissing = true;
    },
  };
}

const slot = (
  name: string,
  extra: Partial<WorktreeEntry> = {},
): WorktreeEntry => ({
  path: `/pool/repo/${name}`,
  locked: false,
  lockReason: undefined,
  branch: undefined,
  ...extra,
});

describe("default command and get", () => {
  it("bare `bs` provisions a detached slot", async () => {
    const h = harness();
    expect((await h.run()).code).toBe(0);
    expect(h.stdout()).toBe("🌳 /pool/repo/cafebabe\n");
  });

  it("get -b prints the branch after the path", async () => {
    const h = harness();
    await h.run("get", "-b", "feat");
    expect(h.stdout()).toBe("🌳 /pool/repo/cafebabe  (feat)\n");
  });

  it("rejects -b with -B", async () => {
    const h = harness();
    const r = await h.run("get", "-b", "a", "-B", "b");
    expect(r.code).not.toBe(0);
    expect(h.stderr()).toContain("cannot be used with");
  });

  it("rejects a positional branch with -b or -B", async () => {
    for (const flag of ["-b", "-B"]) {
      const h = harness();
      const r = await h.run("get", flag, "foo", "existing");
      expect(r.code).not.toBe(0);
      expect(h.stderr()).toContain("cannot be used with");
    }
  });

  it("--no-attach requires --tmux-session", async () => {
    const h = harness();
    const r = await h.run("get", "--no-attach");
    expect(r.code).not.toBe(0);
    expect(h.stderr()).toContain("--tmux-session");
  });

  it("--tmux-session creates a default-named session and attaches", async () => {
    const h = harness();
    await h.run("get", "--tmux-session");
    expect(h.stdout()).toContain("tmux session: \u{1f333} repo (detached)");
    expect(h.tmuxCalls).toEqual([
      "new \u{1f333} repo (detached) /pool/repo/cafebabe",
      "attach \u{1f333} repo (detached)",
    ]);
  });

  it("--tmux-session=NAME with --no-attach uses the name verbatim and does not attach", async () => {
    const h = harness();
    await h.run("get", "--tmux-session=mine", "--no-attach");
    expect(h.tmuxCalls).toEqual(["new mine /pool/repo/cafebabe"]);
  });

  it("checks tmux before provisioning", async () => {
    const h = harness();
    h.setTmuxMissing();
    const r = await h.run("get", "--tmux-session");
    expect(r.code).toBe(1);
    expect(h.stderr()).toContain("error: tmux was not found on PATH");
    expect(h.added()).toBeUndefined();
  });
});

describe("list / ls", () => {
  it("prints a friendly message when the pool is missing", async () => {
    const h = harness();
    await h.run("list");
    expect(h.stdout()).toContain("No worktrees managed for this repository");
  });

  it("`ls` is an alias", async () => {
    const a = harness([slot("one")]);
    const b = harness([slot("one")]);
    await a.run("list");
    await b.run("ls");
    expect(a.stdout()).toBe(b.stdout());
  });

  it("prints badge, path and branch; marks the current slot", async () => {
    const h = harness([
      slot("one", { locked: true }),
      slot("two", { branch: "feat" }),
    ]);
    await h.run("list");
    const lines = h.stdout().trimEnd().split("\n");
    expect(lines[0]).toBe("  locked     /pool/repo/one");
    expect(lines[1]).toBe("  available  /pool/repo/two (feat)");
  });
});

describe("status / lock / unlock / current", () => {
  it("status prints the itemized report", async () => {
    const h = harness([slot("one", { locked: true, lockReason: "agent" })]);
    await h.run("status", "/pool/repo/one");
    expect(h.stdout()).toBe(
      "🌳 /pool/repo/one\nstatus: locked\nlock reason: agent\n",
    );
  });

  it("status outside a slot fails with exit 1", async () => {
    const h = harness([slot("one")]);
    const r = await h.run("status");
    expect(r.code).toBe(1);
    expect(h.stderr()).toContain("not inside a managed bonsai pool slot");
  });

  it("lock/unlock print confirmations", async () => {
    const h = harness([slot("one")]);
    await h.run("lock", "--reason", "why", "/pool/repo/one");
    await h.run("unlock", "/pool/repo/one");
    expect(h.stdout()).toBe(
      "\u{1f512} locked /pool/repo/one\n\u{1f513} unlocked /pool/repo/one\n",
    );
  });

  it("current outside a slot prints guidance and exits 1", async () => {
    const h = harness([slot("one")]);
    const r = await h.run("current");
    expect(r.code).toBe(1);
    expect(h.stdout()).toContain("Not inside a managed bonsai worktree.");
    expect(h.stderr()).toContain("error: not inside a managed bonsai worktree");
  });
});

describe("prune / info / help / logging", () => {
  it("prune with one available slot reports nothing to prune", async () => {
    const h = harness([slot("one")]);
    await h.run("prune");
    expect(h.stdout()).toBe("Nothing to prune (no available slots).\n");
  });

  it("prune reports each pruned slot with its branch", async () => {
    const h = harness([slot("one"), slot("two", { branch: "old" })]);
    await h.run("prune");
    expect(h.stdout()).toBe(
      "\u{1f5d1}\u{fe0f}  pruned /pool/repo/two  (old)\n",
    );
  });

  it("info prints key: value lines with tilde-abbreviated paths", async () => {
    const h = harness();
    await h.run("--log-level", "debug", "info");
    expect(h.stdout()).toBe(
      [
        "version: 9.9.9",
        "log level: debug",
        "log directory: ~/.local/state/bonsai/logs",
        "current log file: ~/.local/state/bonsai/logs/bonsai.log.2026-01-01",
        "managed root: /pool",
        "",
      ].join("\n"),
    );
    expect(h.level()).toBe("debug");
  });

  it("--log-level is accepted after the subcommand and rejects bad values", async () => {
    const ok = harness();
    await ok.run("info", "--log-level", "warn");
    expect(ok.level()).toBe("warn");

    const bad = harness();
    const r = await bad.run("--log-level", "bogus", "help");
    expect(r.code).not.toBe(0);
    expect(bad.stderr()).toContain("bogus");
  });

  it("help prints usage, the about text and the subcommands", async () => {
    const h = harness();
    expect((await h.run("help")).code).toBe(0);
    const text = h.stdout();
    expect(text).toContain("Usage: bs");
    expect(text).toContain("bonsai");
    expect(text).toContain("--log-level");
    for (const c of [
      "get",
      "list",
      "status",
      "current",
      "help",
      "lock",
      "unlock",
      "info",
      "prune",
    ]) {
      expect(text).toContain(c);
    }
  });

  it("logs the CLI invocation at debug level", async () => {
    const h = harness();
    await h.run("help");
    expect(h.logs.some((l) => l.startsWith("CLI invocation: bs"))).toBe(true);
  });
});

describe("formatting", () => {
  it("formatCurrentPath appends the branch in parentheses", () => {
    expect(formatCurrentPath("~/x", undefined)).toBe("~/x");
    expect(stripAnsi(formatCurrentPath("~/x", "feat"))).toBe("~/x  (feat)");
  });

  it("list badges are padded to a fixed width before colouring", () => {
    const rows = (["available", "in use", "locked"] as const).map((s) =>
      stripAnsi(formatListRow(s, "~/p", undefined, false)),
    );
    expect(rows).toEqual([
      "  available  ~/p",
      "  in use     ~/p",
      "  locked     ~/p",
    ]);
    expect(stripAnsi(formatListRow("locked", "~/p", "b", true))).toBe(
      "▶ locked     ~/p (b)",
    );
    // Colour codes are present but never shift the column.
    expect(formatListRow("locked", "~/p", undefined, false)).toContain(
      "\u001b[",
    );
  });

  it("status report omits empty sections and lists itemized files", () => {
    const base = {
      path: "/p",
      branch: undefined,
      status: "in use" as const,
      lockReason: undefined,
      branchState: "3 commits ahead of main, not merged",
      gitStatus: {
        uncommitted: [{ code: " M", path: "a.txt" }],
        untracked: [{ code: "??", path: "b.txt" }],
      },
    };
    expect(formatStatusReport(base, "~/p").map(stripAnsi)).toEqual([
      "🌳 ~/p",
      "status: in use",
      "branch state: 3 commits ahead of main, not merged",
      "",
      "uncommitted changes (1):",
      "   M  a.txt",
      "",
      "untracked files (1):",
      "  ??  b.txt",
    ]);
    expect(
      formatStatusReport(
        { ...base, gitStatus: { uncommitted: [], untracked: [] } },
        "~/p",
      ),
    ).toHaveLength(3);
  });
});
