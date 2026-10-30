import { homedir } from "node:os";
import { join } from "node:path";
import type { LogLocation } from "@bonsai/core";

export interface PathEnv {
  platform: NodeJS.Platform;
  env: NodeJS.ProcessEnv;
  home: string | undefined;
}

function systemEnv(): PathEnv {
  return { platform: process.platform, env: process.env, home: homedir() };
}

function absoluteOrUndefined(p: string | undefined): string | undefined {
  return p?.startsWith("/") ? p : undefined;
}

/**
 * Resolve the platform log directory (`…/bonsai/logs`) without any I/O.
 *
 * Mirrors the previous implementation: the XDG state dir (`$XDG_STATE_HOME`
 * or `~/.local/state`) on Linux, otherwise the local data dir
 * (`~/Library/Application Support` on macOS, `$XDG_DATA_HOME` or
 * `~/.local/share` elsewhere).
 */
export function logDir(sys: PathEnv = systemEnv()): string | undefined {
  const { platform, env, home } = sys;
  let base: string | undefined;
  if (platform === "linux") {
    base =
      absoluteOrUndefined(env.XDG_STATE_HOME) ??
      (home ? join(home, ".local", "state") : undefined);
  } else if (platform === "darwin") {
    base = home ? join(home, "Library", "Application Support") : undefined;
  } else {
    base =
      absoluteOrUndefined(env.XDG_DATA_HOME) ??
      (home ? join(home, ".local", "share") : undefined);
  }
  return base === undefined ? undefined : join(base, "bonsai", "logs");
}

/** `bonsai.log.YYYY-MM-DD` (local date) inside `dir`. No I/O. */
export function currentLogFile(dir: string, now: Date = new Date()): string {
  const y = String(now.getFullYear()).padStart(4, "0");
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return join(dir, `bonsai.log.${y}-${m}-${d}`);
}

/** The `LogLocation` port backed by the platform paths above. */
export function createLogLocation(sys?: PathEnv): LogLocation {
  return {
    dir: () => logDir(sys),
    currentFile: (dir) => currentLogFile(dir),
  };
}
