/** Composition root: wires the core ports to their adapters. */
import { randomUUID } from "node:crypto";
import { copyFile, mkdir, realpath, rm, stat } from "node:fs/promises";
import { homedir } from "node:os";
import type { Deps, Env, Fs, LogLocation, Tmux } from "@bonsai/core";
import { createGit } from "@bonsai/git";
import {
  createLogging,
  createLogLocation,
  type LogLevel,
} from "@bonsai/logging";
import { createTmux } from "@bonsai/tmux";

export interface Runtime {
  deps: Deps;
  tmux: Tmux;
  logs: LogLocation;
  /** Flush and release resources (log file). */
  close(): void;
}

export type RuntimeFactory = (options: { logLevel: LogLevel }) => Runtime;

export const nodeFs: Fs = {
  async exists(path) {
    try {
      await stat(path);
      return true;
    } catch {
      return false;
    }
  },
  realpath: (path) => realpath(path),
  async mkdirp(path) {
    await mkdir(path, { recursive: true });
  },
  copyFile: (src, dest) => copyFile(src, dest),
  removeDir: (path) => rm(path, { recursive: true }),
};

export const nodeEnv: Env = {
  cwd: () => process.cwd(),
  get: (name) => process.env[name],
  homeDir: () => homedir() || undefined,
  randomSlotId: () => randomUUID().slice(0, 8),
};

export const createRuntime: RuntimeFactory = ({ logLevel }) => {
  const logging = createLogging({ level: logLevel });
  return {
    deps: {
      git: createGit(),
      fs: nodeFs,
      env: nodeEnv,
      log: logging.logger,
    },
    tmux: createTmux(),
    logs: createLogLocation(),
    close: () => logging.close(),
  };
};
