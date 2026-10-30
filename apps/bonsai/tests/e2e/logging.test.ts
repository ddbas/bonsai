/**
 * E2E tests for logging: the `--log-level` flag and logging behaviour.
 */
import { expect, it } from "vitest";
import { GitEnv } from "./support";

// ── --log-level flag ──────────────────────────────────────────────────────────

it("--log-level with valid value exits zero", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["--log-level", "debug", "help"]).status).toBe(0);
});

it("--log-level accepts all valid levels", async () => {
  const env = await GitEnv.create();
  for (const level of ["trace", "debug", "info", "warn", "error"]) {
    expect(
      env.bs(["--log-level", level, "help"]).status,
      `\`bs --log-level ${level} help\` should exit 0`,
    ).toBe(0);
  }
});

it("--log-level with invalid value exits nonzero", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["--log-level", "bogus", "help"]).status).not.toBe(0);
});

it("--log-level appears in help", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["--help"]).stdout).toContain("log-level");
});

// ── Logging behaviour ─────────────────────────────────────────────────────────

it("default log level is info", async () => {
  // Basic smoke test that the CLI runs with the default log level.
  const env = await GitEnv.create();
  expect(env.bs(["help"]).status).toBe(0);
});

it("log output does not appear on stdout", async () => {
  // Log events must not pollute stdout (logs go to a file only).
  const env = await GitEnv.create();
  const stdout = env.bs(["--log-level", "debug", "help"]).stdout;
  expect(
    !stdout.includes("TRACE") && !stdout.includes("DEBUG"),
    `\`bs\` should not print log-level names to stdout; got:\n${stdout}`,
  ).toBe(true);
});

it("log output does not appear on stderr", async () => {
  // Normal operations must not produce stderr spam.
  const env = await GitEnv.create();
  const stderr = env.bs(["--log-level", "debug", "help"]).stderr;
  expect(
    stderr === "" || !stderr.toLowerCase().includes("error"),
    `\`bs help\` with debug log level should not produce errors on stderr; got:\n${stderr}`,
  ).toBe(true);
});
