/**
 * E2E tests for the `bs info` command.
 *
 * Verify that `bs info` reports runtime paths and metadata without
 * performing any filesystem writes.
 */
import { expect, it } from "vitest";
import { GitEnv } from "./support";

async function info(args: string[] = []): Promise<string> {
  const env = await GitEnv.create();
  const out = env.bs([...args, "info"]);
  expect(out.status, "`bs info` should exit 0").toBe(0);
  return out.stdout;
}

it("info exits zero", async () => {
  await info();
});

it.each([
  ["version", "version:"],
  ["log level", "log level:"],
  ["log directory", "log directory:"],
  ["current log file", "current log file:"],
  ["managed root", "managed root:"],
])("info prints %s", async (_name, field) => {
  expect(await info(), `\`bs info\` should print '${field}'`).toContain(field);
});

it("info effective log level default", async () => {
  expect(await info()).toContain("log level: info");
});

it("info effective log level debug override", async () => {
  expect(await info(["--log-level", "debug"])).toContain("log level: debug");
});

it("info paths use tilde abbreviation", async () => {
  // At least the log directory is tilde-abbreviated (the managed root in tests
  // is a temp dir that is not under $HOME).
  expect(await info()).toContain("log directory: ~/");
});
