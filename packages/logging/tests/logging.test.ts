import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  createLogging,
  createLogLocation,
  currentLogFile,
  isLogLevel,
  LOG_LEVELS,
  logDir,
  pruneLogs,
} from "../src/index.ts";

const dirs: string[] = [];
function tmp(): string {
  const d = mkdtempSync(join(tmpdir(), "bonsai-log-"));
  dirs.push(d);
  return d;
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

describe("logDir", () => {
  it("resolves the XDG state dir on linux", () => {
    expect(logDir({ platform: "linux", env: {}, home: "/home/me" })).toBe(
      "/home/me/.local/state/bonsai/logs",
    );
    expect(
      logDir({
        platform: "linux",
        env: { XDG_STATE_HOME: "/x/state" },
        home: "/home/me",
      }),
    ).toBe("/x/state/bonsai/logs");
  });

  it("ignores a relative XDG_STATE_HOME", () => {
    expect(
      logDir({
        platform: "linux",
        env: { XDG_STATE_HOME: "rel" },
        home: "/home/me",
      }),
    ).toBe("/home/me/.local/state/bonsai/logs");
  });

  it("falls back to the local data dir on macOS", () => {
    expect(logDir({ platform: "darwin", env: {}, home: "/Users/me" })).toBe(
      "/Users/me/Library/Application Support/bonsai/logs",
    );
  });

  it("returns a path for the real environment and undefined without a home", () => {
    expect(logDir()).toBeTruthy();
    expect(
      logDir({ platform: "linux", env: {}, home: undefined }),
    ).toBeUndefined();
  });
});

describe("currentLogFile", () => {
  it("is bonsai.log.YYYY-MM-DD for the local date", () => {
    expect(basename(currentLogFile("/tmp/logs", new Date(2026, 6, 4)))).toBe(
      "bonsai.log.2026-07-04",
    );
  });

  it("uses today's date by default", () => {
    expect(basename(currentLogFile("/tmp/logs"))).toMatch(
      /^bonsai\.log\.\d{4}-\d{2}-\d{2}$/,
    );
  });

  it("the LogLocation port wires both together", () => {
    const loc = createLogLocation({ platform: "linux", env: {}, home: "/h" });
    const dir = loc.dir() as string;
    expect(dir).toBe("/h/.local/state/bonsai/logs");
    expect(loc.currentFile(dir).startsWith(`${dir}/bonsai.log.`)).toBe(true);
  });
});

describe("levels", () => {
  it("accepts exactly trace/debug/info/warn/error", () => {
    expect([...LOG_LEVELS]).toEqual([
      "trace",
      "debug",
      "info",
      "warn",
      "error",
    ]);
    expect(isLogLevel("debug")).toBe(true);
    expect(isLogLevel("bogus")).toBe(false);
  });
});

describe("pruneLogs", () => {
  function seed(dir: string, count: number): void {
    for (let i = 0; i < count; i++) {
      const p = join(
        dir,
        `bonsai.log.2026-07-${String(i + 1).padStart(2, "0")}`,
      );
      writeFileSync(p, `log ${i}`);
      const t = new Date(2026, 0, 1, 0, 0, i);
      utimesSync(p, t, t);
    }
  }

  it("keeps exactly the N most recently modified files", () => {
    const dir = tmp();
    seed(dir, 10);
    pruneLogs(dir, 7);
    const left = readdirSync(dir).sort();
    expect(left).toHaveLength(7);
    expect(left[0]).toBe("bonsai.log.2026-07-04");
  });

  it("ignores non-matching files", () => {
    const dir = tmp();
    seed(dir, 2);
    writeFileSync(join(dir, "other_file.txt"), "other");
    writeFileSync(join(dir, "readme.md"), "readme");
    pruneLogs(dir, 1);
    const left = readdirSync(dir);
    expect(left).toHaveLength(3);
    expect(left).toContain("other_file.txt");
    expect(left).toContain("readme.md");
  });

  it("tolerates a missing directory", () => {
    expect(() => pruneLogs(join(tmp(), "nope"), 7)).not.toThrow();
  });
});

describe("createLogging", () => {
  it("writes entries at or above the level to the daily file, never to stdout/stderr", () => {
    const dir = join(tmp(), "nested", "logs");
    const logging = createLogging({ level: "info", dir });
    logging.logger.debug("hidden");
    logging.logger.info("visible info");
    logging.logger.warn("visible warn");
    logging.close();

    const file = currentLogFile(dir);
    expect(existsSync(file)).toBe(true);
    const lines = readFileSync(file, "utf8").trim().split("\n");
    const entries = lines.map((l) => JSON.parse(l));
    expect(entries.map((e) => [e.level, e.msg])).toEqual([
      ["info", "visible info"],
      ["warn", "visible warn"],
    ]);
    expect(typeof entries[0].time).toBe("string");
  });

  it("debug level records debug messages", () => {
    const dir = tmp();
    const logging = createLogging({ level: "debug", dir });
    logging.logger.debug("detail");
    logging.close();
    expect(readFileSync(currentLogFile(dir), "utf8")).toContain("detail");
  });

  it("appends across runs", () => {
    const dir = tmp();
    for (const m of ["one", "two"]) {
      const l = createLogging({ level: "info", dir });
      l.logger.info(m);
      l.close();
    }
    expect(
      readFileSync(currentLogFile(dir), "utf8").trim().split("\n"),
    ).toHaveLength(2);
  });

  it("degrades to a no-op logger when the directory cannot be created", () => {
    const base = tmp();
    const blocker = join(base, "file");
    writeFileSync(blocker, "x");
    const logging = createLogging({
      level: "info",
      dir: join(blocker, "logs"),
    });
    expect(() => {
      logging.logger.info("ignored");
      logging.close();
    }).not.toThrow();
    mkdirSync(join(base, "ok"));
  });
});
