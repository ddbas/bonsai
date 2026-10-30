/**
 * E2E tests for `bs status`.
 *
 * All tests use `GitEnv` to run in a fully isolated Docker-backed git
 * environment. Host `~/.bonsai` is never touched.
 */
import {
  closeSync,
  mkdirSync,
  openSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { basename, join } from "node:path";
import { expect, it } from "vitest";
import { expectFail, expectOk, GitEnv, hostGit } from "./support";

/** `bs status <slot>` expecting success; returns stdout. */
function statusOf(env: GitEnv, slot: string): string {
  const out = env.bs(["status", slot]);
  expectOk(out, "bs status");
  return out.stdout;
}

// ── explicit path invocation ──────────────────────────────────────────────────

/** `bs status <path>` on a clean, unlocked slot reports `available`. */
it("status explicit path reports available", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  const stdout = statusOf(env, slot);
  expect(stdout).toContain("available");
  expect(stdout, "output should contain the slot directory name").toContain(
    basename(slot),
  );
});

// ── default-to-current-slot invocation ────────────────────────────────────────

/** `bs status` with no arguments, run from inside a managed slot, reports on that slot. */
it("status defaults to current slot", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  const out = env.bsFrom(slot, ["status"]);
  expectOk(out, "bs status (no path)");
  expect(out.stdout).toContain(basename(slot));
});

/** Run from a subdirectory of a managed slot, it reports on the containing slot. */
it("status defaults to current slot from subdirectory", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  const subdir = join(slot, "subdir");
  mkdirSync(subdir);

  const out = env.bsFrom(subdir, ["status"]);
  expectOk(out, "bs status from a slot subdirectory");
  expect(out.stdout).toContain(basename(slot));
});

// ── error when CWD is not in a managed slot and no path given ─────────────────

it("status no path outside managed slot errors", async () => {
  const env = await GitEnv.create();
  env.runGet();

  // repoPath is not itself inside the managed pool.
  const out = env.bsFrom(env.repoPath, ["status"]);

  expectFail(out, "bs status with no path outside a managed slot");
  expect(out.stderr).toContain("not inside a managed bonsai pool slot");
});

// ── error for a path outside the pool ─────────────────────────────────────────

it("status path outside pool errors", async () => {
  const env = await GitEnv.create();
  env.runGet();

  const out = env.bs(["status", env.repoPath]);

  expectFail(out, "bs status with a path outside the pool");
  expect(out.stderr).toContain("not a bonsai-managed pool slot");
});

it("status nonexistent path errors", async () => {
  const env = await GitEnv.create();
  env.runGet();

  const missing = join(env.bonsaiPath, "does-not-exist-xyz");
  const out = env.bs(["status", missing]);

  expectFail(out, "bs status with a nonexistent path");
  expect(out.stderr).toContain("does not exist");
});

// ── classification priority: locked > in use > available ──────────────────────

it("status clean unlocked slot is available", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  expect(statusOf(env, slot)).toContain("available");
});

it("status dirty slot is in use", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  writeFileSync(join(slot, "dirty.txt"), "dirty");

  const stdout = statusOf(env, slot);
  expect(stdout).toContain("in use");
  expect(stdout).not.toContain("available");
});

it("status locked and dirty slot is locked", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  writeFileSync(join(slot, "dirty.txt"), "dirty");

  expect(env.bs(["lock", slot]).status).toBe(0);

  const stdout = statusOf(env, slot);
  expect(stdout).toContain("locked");
  expect(stdout).not.toContain("in use");
});

// ── lock reason display ───────────────────────────────────────────────────────

it("status shows lock reason when present", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  const reason = "reserved for agent build";
  expect(env.bs(["lock", "--reason", reason, slot]).status).toBe(0);

  expect(statusOf(env, slot)).toContain(reason);
});

it("status shows no reason when absent", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  expect(env.bs(["lock", slot]).status).toBe(0);

  const stdout = statusOf(env, slot);
  expect(stdout).toContain("locked");
  const lower = stdout.toLowerCase();
  expect(
    lower.includes("none") || lower.includes("no reason"),
    `expected an indication that no reason was given, got: ${stdout}`,
  ).toBe(true);
});

// ── itemized uncommitted/untracked file lists ─────────────────────────────────

it("status lists individual uncommitted and untracked files", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  // Modify a tracked file.
  const tracked = hostGit(slot, ["ls-files"]);
  const firstTracked = tracked.stdout.split("\n")[0];
  expect(
    firstTracked,
    "repo should have at least one tracked file",
  ).toBeTruthy();
  writeFileSync(join(slot, firstTracked as string), "modified content");

  // Add an untracked file.
  writeFileSync(join(slot, "untracked.txt"), "hello");

  const stdout = statusOf(env, slot);
  expect(stdout).toContain(firstTracked);
  expect(stdout).toContain("untracked.txt");
});

it("status clean slot has no file sections", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  const stdout = statusOf(env, slot);
  expect(
    !stdout.includes("uncommitted changes") &&
      !stdout.includes("untracked files"),
    `a clean slot should not print uncommitted/untracked sections, got: ${stdout}`,
  ).toBe(true);
});

// ── branch state ──────────────────────────────────────────────────────────────

/** An unpushed branch with commits is `in use` and the report explains why. */
it("status reports unpushed commits branch state", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  await env.git("checkout", "-q", "-b", "feat");
  await env.makeCommit("a.txt", "a");
  await env.makeCommit("b.txt", "b");
  await env.makeCommit("c.txt", "c");
  await env.git("checkout", "-q", "main");
  const slot = env.runGetBranch("feat");

  const stdout = env.bs(["status", slot]).stdout;
  expect(stdout).toContain("in use");
  expect(stdout).toContain("3 commits ahead of origin/main");
  expect(stdout).toContain("no upstream configured");
  expect(stdout).toContain("-u");
});

/** A squash-merged branch whose remote branch is gone is `available`. */
it("status reports merged remote gone", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  await env.git("checkout", "-q", "-b", "feat");
  await env.makeCommit("f.txt", "f");
  await env.git("push", "-u", "origin", "feat");
  await env.git("checkout", "-q", "main");
  await env.git("merge", "--squash", "feat");
  await env.git("commit", "-q", "-m", "squash");
  await env.git("push", "origin", "main");
  await env.git("push", "origin", "--delete", "feat");
  await env.fixPermissions();
  const slot = env.runGetBranch("feat");

  const stdout = env.bs(["status", slot]).stdout;
  expect(stdout).toContain("available");
  expect(stdout).toContain("remote branch is gone");
});

/** A file inside a new untracked directory makes the slot `in use`. */
it("status untracked file in new directory is in use", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  mkdirSync(join(slot, "newdir/sub"), { recursive: true });
  writeFileSync(join(slot, "newdir/sub/f.txt"), "x");
  expect(env.statusWord(slot)).toBe("in use");
  expect(env.bs(["status", slot]).stdout).toContain("newdir/sub/f.txt");
});

/** Open processes no longer affect classification, and `bs status` does not list them. */
it("status ignores open process", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  const filePath = join(slot, "held_open.txt");
  writeFileSync(filePath, "data");
  const fd = openSync(filePath, "r");
  try {
    // `held_open.txt` is untracked; remove it so only the open handle (which
    // stays valid after unlink) remains.
    unlinkSync(filePath);

    const stdout = statusOf(env, slot);
    expect(stdout).toContain("available");
    expect(stdout).not.toContain("open processes");
  } finally {
    closeSync(fd);
  }
});
