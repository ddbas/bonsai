/**
 * E2E tests for `bs get --tmux-session` / `--no-attach`.
 *
 * These exercise real `tmux` session creation/reuse against a throwaway tmux
 * server (a unique `-L <socket>` per test, selected via the
 * `BONSAI_TMUX_SOCKET` environment variable) so the developer's/CI runner's
 * real tmux server is never touched, and parallel runs never collide.
 *
 * Tests that need tmux are skipped when `tmux` is not found on `PATH`.
 *
 * All tests pass `--no-attach`: `attach-session`/`switch-client` require an
 * interactive controlling terminal, which is not available to a subprocess.
 */
import { randomUUID } from "node:crypto";
import { realpathSync } from "node:fs";
import { expect, it, onTestFinished } from "vitest";
import {
  expectFail,
  expectOk,
  GitEnv,
  hostRun,
  pathFromOutput,
} from "./support";

const tmuxAvailable = (() => {
  try {
    return hostRun("tmux", ["-V"]).status === 0;
  } catch {
    return false;
  }
})();

const tmuxIt = it.skipIf(!tmuxAvailable);

/** A throwaway tmux server isolated via a unique `-L <socket>` name. */
class TmuxSocket {
  readonly name: string;

  constructor(label: string) {
    this.name = `bonsai-test-${label}-${randomUUID()}`;
    onTestFinished(() => {
      hostRun("tmux", ["-L", this.name, "kill-server"]);
    });
  }

  hasSession(session: string): boolean {
    return (
      hostRun("tmux", ["-L", this.name, "has-session", "-t", session])
        .status === 0
    );
  }

  panePath(session: string): string {
    return hostRun("tmux", [
      "-L",
      this.name,
      "display-message",
      "-p",
      "-t",
      session,
      "#{pane_current_path}",
    ]).stdout.trim();
  }

  listSessions(): string[] {
    return hostRun("tmux", [
      "-L",
      this.name,
      "list-sessions",
      "-F",
      "#{session_name}",
    ])
      .stdout.split("\n")
      .map((l) => l.trim())
      .filter((l) => l !== "");
  }
}

/** The first line of `bs get`'s stdout (`🌳 <path> [(branch)]`). */
function firstLine(raw: string): string {
  return raw.split("\n")[0] ?? "";
}

/** Session name printed by `bs get --tmux-session` (`tmux session: <name>`). */
function sessionNameFromOutput(raw: string): string {
  for (const line of raw.split("\n")) {
    const idx = line.indexOf("tmux session: ");
    if (idx >= 0) return line.slice(idx + "tmux session: ".length).trim();
  }
  throw new Error(`no tmux session line found in stdout:\n${raw}`);
}

function canonical(p: string): string {
  try {
    return realpathSync(p);
  } catch {
    return p;
  }
}

// ── creation ──────────────────────────────────────────────────────────────────

tmuxIt("get --tmux-session creates default-named session", async () => {
  const env = await GitEnv.create();
  const socket = new TmuxSocket("default-name");

  const out = env.bs(["get", "--tmux-session", "--no-attach"], {
    env: { BONSAI_TMUX_SOCKET: socket.name },
  });
  expectOk(out, "bs get --tmux-session --no-attach");

  const sessionName = sessionNameFromOutput(out.stdout);
  // Default naming convention: 🌳 <repo-name> (detached) — no branch flags
  // were passed, so branch-display must be the literal `detached`.
  expect(
    sessionName,
    "expected default session name to end in (detached)",
  ).toContain("(detached)");
  expect(
    socket.hasSession(sessionName),
    `expected tmux session ${sessionName} to exist on socket ${socket.name}`,
  ).toBe(true);
});

tmuxIt("get --tmux-session session rooted at slot path", async () => {
  const env = await GitEnv.create();
  const socket = new TmuxSocket("cwd");

  const out = env.bs(["get", "--tmux-session", "--no-attach"], {
    env: { BONSAI_TMUX_SOCKET: socket.name },
  });
  expect(out.status).toBe(0);

  const sessionName = sessionNameFromOutput(out.stdout);
  const slotPath = pathFromOutput(firstLine(out.stdout));

  expect(canonical(socket.panePath(sessionName))).toBe(canonical(slotPath));
});

tmuxIt("get --tmux-session default name reflects branch", async () => {
  const env = await GitEnv.create();
  const socket = new TmuxSocket("branch-name");

  const out = env.bs(
    ["get", "-b", "my-feature", "--tmux-session", "--no-attach"],
    { env: { BONSAI_TMUX_SOCKET: socket.name } },
  );
  expect(out.status).toBe(0);

  const sessionName = sessionNameFromOutput(out.stdout);
  expect(
    sessionName,
    "expected session name to contain (my-feature)",
  ).toContain("(my-feature)");
});

// ── reuse ─────────────────────────────────────────────────────────────────────

tmuxIt("get --tmux-session rerun reuses existing session", async () => {
  const env = await GitEnv.create();
  const socket = new TmuxSocket("reuse");
  const opts = { env: { BONSAI_TMUX_SOCKET: socket.name } };

  const first = env.bs(["get", "--tmux-session", "--no-attach"], opts);
  expect(first.status).toBe(0);
  const firstName = sessionNameFromOutput(first.stdout);

  const second = env.bs(["get", "--tmux-session", "--no-attach"], opts);
  expectOk(second, "rerunning bs get --tmux-session");
  const secondName = sessionNameFromOutput(second.stdout);

  expect(secondName).toBe(firstName);
  expect(socket.hasSession(firstName)).toBe(true);

  // Exactly one session should exist — no duplicate was created.
  const names = socket.listSessions();
  expect(names.length, `expected exactly one tmux session, got: ${names}`).toBe(
    1,
  );
});

// ── custom name ───────────────────────────────────────────────────────────────

tmuxIt("get --tmux-session custom name used verbatim", async () => {
  const env = await GitEnv.create();
  const socket = new TmuxSocket("custom-name");

  const out = env.bs(
    ["get", "--tmux-session=my-custom-session", "--no-attach"],
    {
      env: { BONSAI_TMUX_SOCKET: socket.name },
    },
  );

  expect(out.status).toBe(0);
  expect(sessionNameFromOutput(out.stdout)).toBe("my-custom-session");
  expect(socket.hasSession("my-custom-session")).toBe(true);
});

// ── --no-attach ───────────────────────────────────────────────────────────────

tmuxIt("get --tmux-session --no-attach creates without attaching", async () => {
  const env = await GitEnv.create();
  const socket = new TmuxSocket("no-attach");

  // `--no-attach` must never invoke `switch-client`/`attach-session`, so this
  // must complete (and exit 0) even with no controlling terminal.
  const out = env.bs(["get", "--tmux-session", "--no-attach"], {
    env: { BONSAI_TMUX_SOCKET: socket.name },
  });

  expect(out.status).toBe(0);
  expect(socket.hasSession(sessionNameFromOutput(out.stdout))).toBe(true);
});

it("get --no-attach without --tmux-session fails at parse time", async () => {
  // Pure argument parsing — no tmux invocation happens, so no skip needed.
  const env = await GitEnv.create();

  const out = env.bs(["get", "--no-attach"]);

  expectFail(out, "bs get --no-attach (without --tmux-session)");
  expect(
    out.stderr.includes("tmux-session") || out.stderr.includes("tmux_session"),
    `expected usage error to mention --tmux-session, got: ${out.stderr}`,
  ).toBe(true);
});
