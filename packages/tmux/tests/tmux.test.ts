import { BonsaiError } from "@bonsai/core";
import { describe, expect, it } from "vitest";
import { createTmux, type TmuxProcess, type TmuxResult } from "../src/index.ts";

function fakeProcess(
  opts: {
    capture?: (args: string[]) => Promise<TmuxResult>;
    inherit?: (args: string[]) => Promise<number>;
  } = {},
) {
  const captured: string[][] = [];
  const inherited: string[][] = [];
  const process: TmuxProcess = {
    capture: async (args) => {
      captured.push(args);
      return (await opts.capture?.(args)) ?? { code: 0, stderr: "" };
    },
    inherit: async (args) => {
      inherited.push(args);
      return (await opts.inherit?.(args)) ?? 0;
    },
  };
  return { process, captured, inherited };
}

const enoent = Object.assign(new Error("spawn tmux ENOENT"), {
  code: "ENOENT",
});

describe("socket selection", () => {
  it("pins every call to BONSAI_TMUX_SOCKET with -L", async () => {
    const f = fakeProcess();
    const tmux = createTmux({
      env: { BONSAI_TMUX_SOCKET: "sock" },
      process: f.process,
    });
    await tmux.checkAvailable();
    await tmux.hasSession("s");
    await tmux.newSession("s", "/slot");
    expect(f.captured).toEqual([
      ["-L", "sock", "-V"],
      ["-L", "sock", "has-session", "-t", "s"],
      ["-L", "sock", "new-session", "-d", "-s", "s", "-c", "/slot"],
    ]);
  });

  it("uses the default socket when unset", async () => {
    const f = fakeProcess();
    await createTmux({ env: {}, process: f.process }).hasSession("s");
    expect(f.captured).toEqual([["has-session", "-t", "s"]]);
  });
});

describe("checkAvailable", () => {
  it("gives an actionable error when tmux is missing", async () => {
    const f = fakeProcess({ capture: async () => Promise.reject(enoent) });
    const err = await createTmux({ env: {}, process: f.process })
      .checkAvailable()
      .catch((e) => e);
    expect(err).toBeInstanceOf(BonsaiError);
    expect(err.message).toContain("tmux was not found on PATH");
  });

  it("any exit status counts as available", async () => {
    const f = fakeProcess({ capture: async () => ({ code: 1, stderr: "" }) });
    await expect(
      createTmux({ env: {}, process: f.process }).checkAvailable(),
    ).resolves.toBeUndefined();
  });

  it("wraps other spawn failures", async () => {
    const f = fakeProcess({
      capture: async () => Promise.reject(new Error("EPERM")),
    });
    await expect(
      createTmux({ env: {}, process: f.process }).checkAvailable(),
    ).rejects.toThrow("failed to check for `tmux` on PATH: EPERM");
  });
});

describe("hasSession / newSession", () => {
  it("hasSession reflects the exit status", async () => {
    const yes = fakeProcess();
    expect(
      await createTmux({ env: {}, process: yes.process }).hasSession("s"),
    ).toBe(true);
    const no = fakeProcess({ capture: async () => ({ code: 1, stderr: "" }) });
    expect(
      await createTmux({ env: {}, process: no.process }).hasSession("s"),
    ).toBe(false);
  });

  it("newSession surfaces stderr on failure", async () => {
    const f = fakeProcess({
      capture: async () => ({ code: 1, stderr: "duplicate session\n" }),
    });
    await expect(
      createTmux({ env: {}, process: f.process }).newSession("s", "/x"),
    ).rejects.toThrow("`tmux new-session` failed: duplicate session");
  });
});

describe("attach", () => {
  it("uses attach-session outside tmux", async () => {
    const f = fakeProcess();
    await createTmux({ env: {}, process: f.process }).attach("s");
    expect(f.inherited).toEqual([["attach-session", "-t", "s"]]);
  });

  it("uses switch-client inside tmux", async () => {
    const f = fakeProcess();
    await createTmux({
      env: { TMUX: "/tmp/tmux-1/default,1,0" },
      process: f.process,
    }).attach("s");
    expect(f.inherited).toEqual([["switch-client", "-t", "s"]]);
  });

  it("fails when tmux exits non-zero", async () => {
    const f = fakeProcess({ inherit: async () => 1 });
    await expect(
      createTmux({ env: {}, process: f.process }).attach("s"),
    ).rejects.toThrow("failed to attach to tmux session `s`");
  });
});
