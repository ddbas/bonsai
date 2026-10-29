import { mkdirSync, readdirSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";
import type { Logger } from "@bonsai/core";
import pino from "pino";
import { currentLogFile, logDir } from "./paths.ts";

export const LOG_LEVELS = ["trace", "debug", "info", "warn", "error"] as const;
export type LogLevel = (typeof LOG_LEVELS)[number];

export function isLogLevel(value: string): value is LogLevel {
  return (LOG_LEVELS as readonly string[]).includes(value);
}

/** Number of daily log files kept in the log directory. */
export const LOG_RETENTION = 7;

/** Keep only the `retention` most recently modified `bonsai.log.*` files. */
export function pruneLogs(dir: string, retention: number): void {
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch {
    return;
  }
  const files: { path: string; mtime: number }[] = [];
  for (const name of names) {
    if (!name.startsWith("bonsai.log.")) continue;
    const path = join(dir, name);
    try {
      files.push({ path, mtime: statSync(path).mtimeMs });
    } catch {
      // vanished or unreadable: skip
    }
  }
  files.sort((a, b) => b.mtime - a.mtime);
  for (const file of files.slice(retention)) {
    try {
      rmSync(file.path);
    } catch {
      // best-effort
    }
  }
}

const NOOP_LOGGER: Logger = {
  debug() {},
  info() {},
  warn() {},
  error() {},
};

export interface LoggingOptions {
  level: LogLevel;
  /** Override the log directory (tests). */
  dir?: string;
}

export interface Logging {
  logger: Logger;
  /** Flush and release the log file. */
  close(): void;
}

/**
 * Set up pino logging to a daily log file in the platform log directory.
 *
 * Best-effort: if the directory cannot be created or the file opened, a
 * no-op logger is returned and the CLI proceeds without file logging.
 * Nothing is ever written to stdout/stderr.
 */
export function createLogging(options: LoggingOptions): Logging {
  const dir = options.dir ?? logDir();
  if (dir === undefined) return { logger: NOOP_LOGGER, close() {} };

  try {
    mkdirSync(dir, { recursive: true });
    pruneLogs(dir, LOG_RETENTION);

    const destination = pino.destination({
      dest: currentLogFile(dir),
      sync: true,
      append: true,
    });
    const base = pino(
      {
        level: options.level,
        base: null,
        timestamp: pino.stdTimeFunctions.isoTime,
        formatters: { level: (label) => ({ level: label }) },
      },
      destination,
    );
    return {
      logger: {
        debug: (m) => base.debug(m),
        info: (m) => base.info(m),
        warn: (m) => base.warn(m),
        error: (m) => base.error(m),
      },
      close: () => destination.flushSync(),
    };
  } catch {
    return { logger: NOOP_LOGGER, close() {} };
  }
}
