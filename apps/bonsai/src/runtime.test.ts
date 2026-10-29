import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { nodeEnv, nodeFs } from "./runtime.ts";

const dirs: string[] = [];
function tmp(): string {
  const d = mkdtempSync(join(tmpdir(), "bonsai-rt-"));
  dirs.push(d);
  return d;
}
afterEach(() => {
  for (const d of dirs.splice(0)) {
    try {
      chmodSync(d, 0o700);
    } catch {}
    rmSync(d, { recursive: true, force: true });
  }
});

describe("nodeEnv", () => {
  it("slot ids are 8 lowercase hex characters and differ between calls", () => {
    const ids = Array.from({ length: 20 }, () => nodeEnv.randomSlotId());
    for (const id of ids) expect(id).toMatch(/^[0-9a-f]{8}$/);
    expect(new Set(ids).size).toBe(20);
  });

  it("reads cwd, env vars and the home directory", () => {
    expect(nodeEnv.cwd()).toBe(process.cwd());
    expect(nodeEnv.get("PATH")).toBe(process.env.PATH);
    expect(nodeEnv.get("DEFINITELY_NOT_SET_BONSAI")).toBeUndefined();
    expect(nodeEnv.homeDir()).toBeTruthy();
  });
});

describe("nodeFs", () => {
  it("exists / realpath resolve symlinks and reject missing paths", async () => {
    const dir = tmp();
    mkdirSync(join(dir, "real"));
    symlinkSync(join(dir, "real"), join(dir, "link"));
    expect(await nodeFs.exists(join(dir, "link"))).toBe(true);
    expect(await nodeFs.exists(join(dir, "nope"))).toBe(false);
    expect(await nodeFs.realpath(join(dir, "link"))).toBe(
      await nodeFs.realpath(join(dir, "real")),
    );
    await expect(nodeFs.realpath(join(dir, "nope"))).rejects.toThrow();
  });

  it("mkdirp is idempotent and copyFile copies contents", async () => {
    const dir = tmp();
    await nodeFs.mkdirp(join(dir, "a/b"));
    await nodeFs.mkdirp(join(dir, "a/b"));
    writeFileSync(join(dir, "src.txt"), "hello");
    await nodeFs.copyFile(join(dir, "src.txt"), join(dir, "a/b/dest.txt"));
    expect(readFileSync(join(dir, "a/b/dest.txt"), "utf8")).toBe("hello");
  });

  it("removeDir deletes recursively and rejects a missing path", async () => {
    const dir = tmp();
    mkdirSync(join(dir, "x/y"), { recursive: true });
    writeFileSync(join(dir, "x/y/f"), "1");
    await nodeFs.removeDir(join(dir, "x"));
    expect(existsSync(join(dir, "x"))).toBe(false);
    await expect(nodeFs.removeDir(join(dir, "x"))).rejects.toThrow();
  });
});
