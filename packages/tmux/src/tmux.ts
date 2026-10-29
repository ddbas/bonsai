import { type SpawnOptions, spawn } from "node:child_process";
import { BonsaiError, type Tmux } from "@bonsai/core";

export interface TmuxResult {
  code: number;
  stderr: string;
}

/** Process seam so the adapter can be unit-tested without a tmux binary. */
export interface TmuxProcess {
  /** Run `tmux <args>` capturing output. Rejects with `ENOENT` when missing. */
  capture(args: string[]): Promise<TmuxResult>;
  /** Run `tmux <args>` attached to the invoking terminal. */
  inherit(args: string[]): Promise<number>;
}

export interface TmuxOptions {
  env?: NodeJS.ProcessEnv;
  process?: TmuxProcess;
}

function spawnTmux(
  args: string[],
  options: SpawnOptions,
): Promise<{ code: number; stderr: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn("tmux", args, options);
    let stderr = "";
    child.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", reject);
    child.on("close", (code) => resolve({ code: code ?? 1, stderr }));
  });
}

const nodeProcess: TmuxProcess = {
  capture: (args) => spawnTmux(args, { stdio: ["ignore", "ignore", "pipe"] }),
  inherit: async (args) => (await spawnTmux(args, { stdio: "inherit" })).code,
};

/**
 * tmux adapter: implements the core `Tmux` port.
 *
 * When `BONSAI_TMUX_SOCKET` is set every invocation is pinned to that socket
 * via `-L <name>`, so tests can exercise real session creation against a
 * throwaway server instead of the user's real tmux server.
 */
export function createTmux(options: TmuxOptions = {}): Tmux {
  const env = options.env ?? process.env;
  const proc = options.process ?? nodeProcess;

  const socketArgs = (): string[] => {
    const socket = env.BONSAI_TMUX_SOCKET;
    return socket === undefined ? [] : ["-L", socket];
  };

  return {
    async checkAvailable() {
      try {
        await proc.capture([...socketArgs(), "-V"]);
      } catch (e) {
        if ((e as NodeJS.ErrnoException).code === "ENOENT") {
          throw new BonsaiError(
            "tmux was not found on PATH; install tmux to use --tmux-session, or omit the flag to skip tmux integration",
          );
        }
        throw new BonsaiError(
          `failed to check for \`tmux\` on PATH: ${(e as Error).message}`,
        );
      }
    },

    async hasSession(name) {
      try {
        const r = await proc.capture([
          ...socketArgs(),
          "has-session",
          "-t",
          name,
        ]);
        return r.code === 0;
      } catch (e) {
        throw new BonsaiError(
          `failed to spawn \`tmux has-session\`: ${(e as Error).message}`,
        );
      }
    },

    async newSession(name, cwd) {
      let r: TmuxResult;
      try {
        r = await proc.capture([
          ...socketArgs(),
          "new-session",
          "-d",
          "-s",
          name,
          "-c",
          cwd,
        ]);
      } catch (e) {
        throw new BonsaiError(
          `failed to spawn \`tmux new-session\`: ${(e as Error).message}`,
        );
      }
      if (r.code !== 0) {
        throw new BonsaiError(
          `\`tmux new-session\` failed: ${r.stderr.trim()}`,
        );
      }
    },

    async attach(name) {
      // Inside a tmux client `attach-session` would fail; switch instead.
      const insideTmux = env.TMUX !== undefined;
      const args = insideTmux
        ? ["switch-client", "-t", name]
        : ["attach-session", "-t", name];
      let code: number;
      try {
        code = await proc.inherit([...socketArgs(), ...args]);
      } catch (e) {
        throw new BonsaiError(
          `failed to spawn \`tmux switch-client\`/\`tmux attach-session\`: ${(e as Error).message}`,
        );
      }
      if (code !== 0) {
        throw new BonsaiError(`failed to attach to tmux session \`${name}\``);
      }
    },
  };
}
