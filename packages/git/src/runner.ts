import { execFile } from "node:child_process";
import { BonsaiError } from "@bonsai/core";

export interface RunResult {
  /** Process exit code. */
  code: number;
  stdout: string;
  stderr: string;
}

/** Runs `git <args>` and resolves with the result (never rejects on non-zero). */
export type Runner = (args: string[]) => Promise<RunResult>;

/**
 * Environment variables git sets inside hooks. Inherited by children, they
 * would make our git calls operate on the *hook's* repository instead of the
 * one chosen by the working directory, so they are always removed.
 */
export const HOOK_ENV_VARS = [
  "GIT_DIR",
  "GIT_INDEX_FILE",
  "GIT_OBJECT_DIRECTORY",
  "GIT_ALTERNATE_OBJECT_DIRECTORIES",
  "GIT_WORK_TREE",
  "GIT_PREFIX",
  "GIT_INTERNAL_SUPER_PREFIX",
  "GIT_COMMON_DIR",
] as const;

/** `process.env` without hook-injected git variables. */
export function cleanGitEnv(env: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  const out = { ...env };
  for (const v of HOOK_ENV_VARS) delete out[v];
  return out;
}

/** Counting semaphore bounding concurrent git processes. */
export class Semaphore {
  private active = 0;
  private readonly waiters: (() => void)[] = [];

  constructor(private readonly max: number) {}

  async run<T>(task: () => Promise<T>): Promise<T> {
    if (this.active >= this.max) {
      await new Promise<void>((resolve) => this.waiters.push(resolve));
    }
    this.active++;
    try {
      return await task();
    } finally {
      this.active--;
      this.waiters.shift()?.();
    }
  }
}

/** Default runner: async `execFile("git")` with a bounded process pool. */
export function createExecRunner(maxConcurrency: number): Runner {
  const gate = new Semaphore(maxConcurrency);
  return (args) =>
    gate.run(
      () =>
        new Promise<RunResult>((resolve, reject) => {
          execFile(
            "git",
            args,
            {
              env: cleanGitEnv(process.env),
              encoding: "utf8",
              maxBuffer: 256 * 1024 * 1024,
            },
            (error, stdout, stderr) => {
              if (error === null) {
                resolve({ code: 0, stdout, stderr });
              } else if (typeof error.code === "number") {
                resolve({ code: error.code, stdout, stderr });
              } else {
                reject(
                  new BonsaiError(
                    `failed to spawn \`git ${args[0] ?? ""}\`: ${error.message}`,
                  ),
                );
              }
            },
          );
        }),
    );
}
