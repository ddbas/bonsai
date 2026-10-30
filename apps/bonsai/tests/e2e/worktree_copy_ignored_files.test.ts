/**
 * E2E tests for the `bonsai.copy` git-ignored-file copy feature.
 *
 * Every test uses `GitEnv`: git setup/config runs inside an isolated
 * container, while `bs` runs on the host against the shared bind-mounted repo.
 */
import {
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { expectOk, GitEnv, pathFromOutput } from "./support";

/** A file listed in `bonsai.copy` and present in the origin worktree ends up in a new slot. */
it("new slot receives configured copy", async () => {
  const env = await GitEnv.create();
  await env.git("config", "--add", "bonsai.copy", ".env");

  // `.env` is untracked/ignored in the origin worktree; write it on the host.
  writeFileSync(join(env.repoPath, ".env"), "SECRET=123");

  const slot = env.runGet();

  expect(readFileSync(join(slot, ".env"), "utf8")).toBe("SECRET=123");
});

/** The same copy also applies when `bs get` reuses an existing available slot. */
it("reused slot receives configured copy", async () => {
  const env = await GitEnv.create();
  await env.git("config", "--add", "bonsai.copy", ".env");

  // First call creates a slot with no `.env` present yet.
  const slot1 = env.runGet();
  expect(
    existsSync(join(slot1, ".env")),
    "no .env in origin worktree yet, so nothing should be copied",
  ).toBe(false);

  // Now the origin gets an `.env`; the slot is reused (still clean).
  writeFileSync(join(env.repoPath, ".env"), "SECRET=456");
  const slot2 = env.runGet();

  expect(slot2, "the same slot should be reused").toBe(slot1);
  expect(readFileSync(join(slot2, ".env"), "utf8")).toBe("SECRET=456");
});

/** A nonexistent `bonsai.copy` entry does not fail `bs get`; others are still copied. */
it("missing entry does not fail get and others still copied", async () => {
  const env = await GitEnv.create();
  await env.git("config", "--add", "bonsai.copy", ".env");
  await env.git("config", "--add", "bonsai.copy", ".env.missing");

  writeFileSync(join(env.repoPath, ".env"), "SECRET=789");
  // Intentionally do not create `.env.missing`.

  const out = env.bs(["get"]);
  expectOk(out, "bs get with a missing bonsai.copy entry");

  const slot = pathFromOutput(out.stdout);
  expect(
    existsSync(join(slot, ".env")),
    "the present entry should still be copied",
  ).toBe(true);
  expect(
    existsSync(join(slot, ".env.missing")),
    "the missing entry should simply be absent from the slot",
  ).toBe(false);
});

/** With `bonsai.copy` unset, `bs get` behaves exactly as before (regression guard). */
it("unset bonsai.copy is a regression no-op", async () => {
  const env = await GitEnv.create();

  const slot = env.runGet();

  expect(existsSync(slot)).toBe(true);
  expect(
    existsSync(join(slot, "README.md")),
    "the tracked README.md from init should still be present",
  ).toBe(true);
});

/** A nested relative `bonsai.copy` entry is copied with parent directories created. */
it("nested relative path entry creates parent dirs", async () => {
  const env = await GitEnv.create();
  await env.git("config", "--add", "bonsai.copy", "config/local.json");

  mkdirSync(join(env.repoPath, "config"), { recursive: true });
  writeFileSync(join(env.repoPath, "config/local.json"), '{"k":1}');

  const slot = env.runGet();

  expect(
    statSync(join(slot, "config")).isDirectory(),
    "config/ should be created in the slot",
  ).toBe(true);
  expect(readFileSync(join(slot, "config/local.json"), "utf8")).toBe('{"k":1}');
});
