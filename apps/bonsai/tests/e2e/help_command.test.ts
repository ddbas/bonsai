/**
 * E2E tests for the `bs help` command.
 *
 * All tests run the binary inside a `GitEnv` so the host machine's git
 * configuration, filesystem, and home directory are never touched.
 */
import { expect, it } from "vitest";
import { GitEnv } from "./support";

// ── bs help ───────────────────────────────────────────────────────────────────

it("help subcommand exits zero", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["help"]).status, "`bs help` should exit 0").toBe(0);
});

it("help subcommand prints program name", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["help"]).stdout).toContain("bs");
});

it("help subcommand prints about text", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["help"]).stdout).toContain("bonsai");
});

it("help subcommand lists help subcommand", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["help"]).stdout).toContain("help");
});

// ── bs --help ─────────────────────────────────────────────────────────────────

it("flag --help exits zero", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["--help"]).status, "`bs --help` should exit 0").toBe(0);
});

it("flag --help prints usage", async () => {
  const env = await GitEnv.create();
  expect(env.bs(["--help"]).stdout.toLowerCase()).toContain("usage");
});

// ── bs (no args) ──────────────────────────────────────────────────────────────

it("no args exits zero", async () => {
  const env = await GitEnv.create();
  expect(
    env.bs().status,
    "`bs` with no arguments should exit 0 (default `get` command)",
  ).toBe(0);
});

it("no args prints tree emoji and path to stdout", async () => {
  const env = await GitEnv.create();
  const stdout = env.bs().stdout;
  expect(stdout).toContain("🌳");
  expect(stdout.trim()).not.toBe("");
});
