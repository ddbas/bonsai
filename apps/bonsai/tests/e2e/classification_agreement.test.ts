/**
 * `bs list`'s badge and `bs status`'s classification must agree for the same
 * slot across representative fixtures (locked, dirty, unmerged-branch,
 * clean) — guarding against the two code paths drifting apart.
 */
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { expectOk, GitEnv } from "./support";

/** Extract the classification word both `bs list` and `bs status` print. */
function extractClassification(stdout: string): string {
  if (stdout.includes("locked")) return "locked";
  if (stdout.includes("in use")) return "in use";
  if (stdout.includes("available")) return "available";
  throw new Error(`no recognizable classification found in: ${stdout}`);
}

function assertListAndStatusAgree(env: GitEnv, slot: string): void {
  const listOut = env.bs(["list"]);
  expectOk(listOut, "bs list");
  const statusOut = env.bs(["status", slot]);
  expectOk(statusOut, "bs status");

  expect(
    extractClassification(listOut.stdout),
    `bs list and bs status must agree\nbs list:\n${listOut.stdout}\nbs status:\n${statusOut.stdout}`,
  ).toBe(extractClassification(statusOut.stdout));
}

it("list and status agree on clean slot", async () => {
  const env = await GitEnv.create();
  assertListAndStatusAgree(env, env.runGet());
});

it("list and status agree on dirty slot", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  writeFileSync(join(slot, "dirty.txt"), "dirty");
  assertListAndStatusAgree(env, slot);
});

it("list and status agree on locked slot", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  expect(env.bs(["lock", slot]).status).toBe(0);
  assertListAndStatusAgree(env, slot);
});

it("list and status agree on locked and dirty slot", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  writeFileSync(join(slot, "dirty.txt"), "dirty");
  expect(env.bs(["lock", slot]).status).toBe(0);
  assertListAndStatusAgree(env, slot);
});

it("list and status agree on unmerged branch slot", async () => {
  const env = await GitEnv.create();
  await env.git("checkout", "-q", "-b", "feat");
  await env.makeCommit("f.txt", "f");
  await env.git("checkout", "-q", "-");
  const slot = env.runGetBranch("feat");
  expect(env.statusWord(slot)).toBe("in use");
  assertListAndStatusAgree(env, slot);
});
