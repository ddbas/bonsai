import { describe, expect, it } from "vitest";
import {
  DETACHED_LABEL,
  defaultSessionName,
  ensureSession,
  resolveSessionName,
  type Tmux,
} from "../src/index.ts";
import { FakeLogger } from "./fakes.ts";

describe("session names", () => {
  it("default name follows the worktree-get convention", () => {
    expect(defaultSessionName("bonsai", "my-feature")).toBe(
      "\u{1f333} bonsai (my-feature)",
    );
  });

  it("uses the detached label when no branch was requested", () => {
    expect(defaultSessionName("bonsai", DETACHED_LABEL)).toBe(
      "\u{1f333} bonsai (detached)",
    );
  });

  it("an empty value derives the default name", () => {
    expect(resolveSessionName("", "bonsai", "main")).toBe(
      defaultSessionName("bonsai", "main"),
    );
  });

  it("a non-empty value is used verbatim", () => {
    expect(resolveSessionName("my-custom-session", "bonsai", "main")).toBe(
      "my-custom-session",
    );
  });
});

describe("ensureSession", () => {
  function fakeTmux(existing: boolean) {
    const calls: string[] = [];
    const tmux: Tmux = {
      checkAvailable: async () => {},
      hasSession: async () => existing,
      newSession: async (name, cwd) => {
        calls.push(`new ${name} ${cwd}`);
      },
      attach: async () => {},
    };
    return { tmux, calls };
  }

  it("creates a session rooted at cwd when missing", async () => {
    const { tmux, calls } = fakeTmux(false);
    await ensureSession(tmux, new FakeLogger(), "s", "/slot");
    expect(calls).toEqual(["new s /slot"]);
  });

  it("reuses an existing session", async () => {
    const { tmux, calls } = fakeTmux(true);
    await ensureSession(tmux, new FakeLogger(), "s", "/slot");
    expect(calls).toEqual([]);
  });
});
