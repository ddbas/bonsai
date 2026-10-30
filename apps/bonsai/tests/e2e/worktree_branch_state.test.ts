/**
 * E2E tests for branch-state-based slot availability (`bs get`, `bs list`,
 * `bs status`), using a bare `origin` inside the `GitEnv` bind mount.
 */
import { spawn } from "node:child_process";
import { expect, it } from "vitest";
import { GitEnv, hostGit } from "./support";

/** Create `feat` with one commit, pushed with upstream; leave the repo on `main`. */
async function pushedFeature(env: GitEnv): Promise<void> {
  await env.git("checkout", "-q", "-b", "feat");
  await env.makeCommit("f.txt", "f");
  await env.git("push", "-u", "origin", "feat");
  await env.git("checkout", "-q", "main");
}

async function deleteRemoteFeat(env: GitEnv): Promise<void> {
  await env.git("push", "origin", "--delete", "feat");
  await env.fixPermissions();
}

it("fresh and detached at merged are available", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  const slot = env.runGet();
  expect(env.statusWord(slot)).toBe("available");
  const slot2 = env.runGet();
  expect(slot2, "available slot should be reused").toBe(slot);
});

it("fast-forward merged branch is available even with remote", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  await pushedFeature(env);
  await env.git("merge", "--ff-only", "feat");
  await env.git("push", "origin", "main");
  await env.fixPermissions();
  const slot = env.runGetBranch("feat");
  expect(env.statusWord(slot)).toBe("available");
});

it("squash merged remote gone is available", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  await pushedFeature(env);
  await env.git("merge", "--squash", "feat");
  await env.git("commit", "-q", "-m", "squash");
  await env.git("push", "origin", "main");
  await deleteRemoteFeat(env);
  // main moves on after the squash merge.
  await env.makeCommit("later.txt", "later");
  await env.git("push", "origin", "main");
  const slot = env.runGetBranch("feat");
  expect(env.statusWord(slot)).toBe("available");
});

it("rebase merged remote gone is available", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  await pushedFeature(env);
  await env.makeCommit("other.txt", "o");
  await env.git("cherry-pick", "feat");
  await env.git("push", "origin", "main");
  await deleteRemoteFeat(env);
  const slot = env.runGetBranch("feat");
  expect(env.statusWord(slot)).toBe("available");
});

it("merged but remote present is in use", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  await pushedFeature(env);
  await env.git("merge", "--squash", "feat");
  await env.git("commit", "-q", "-m", "squash");
  await env.git("push", "origin", "main");
  const slot = env.runGetBranch("feat");
  expect(env.statusWord(slot)).toBe("in use");
});

it("pushed unmerged is in use", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  await pushedFeature(env);
  await deleteRemoteFeat(env);
  // Remote deleted without merging.
  const slot = env.runGetBranch("feat");
  expect(env.statusWord(slot)).toBe("in use");
});

it("never pushed is in use and not reused", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  await env.git("checkout", "-q", "-b", "feat");
  await env.makeCommit("f.txt", "f");
  await env.git("checkout", "-q", "main");
  const slot = env.runGetBranch("feat");
  expect(env.statusWord(slot)).toBe("in use");
  const slot2 = env.runGet();
  expect(slot2, "slot with unmerged work must not be reused").not.toBe(slot);
});

it("detached with own commits is in use", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  const slot = env.runGet();
  const out = hostGit(slot, [
    "-c",
    "user.email=t@example.com",
    "-c",
    "user.name=T",
    "-c",
    "commit.gpgsign=false",
    "commit",
    "--allow-empty",
    "-m",
    "own",
  ]);
  expect(out.status, out.stderr).toBe(0);
  expect(env.statusWord(slot)).toBe("in use");
});

it("unresolvable default ref is in use", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  // Remote exists but no origin/main or origin/HEAD resolves.
  await env.git("remote", "set-head", "origin", "-d");
  await env.git("update-ref", "-d", "refs/remotes/origin/main");
  const slot = env.runGet();
  expect(env.statusWord(slot)).toBe("in use");
  const out = env.bs(["status", slot]);
  expect(out.stdout).toContain("default branch could not be determined");
});

it("clean merged slot with process is available and reused", async () => {
  const env = await GitEnv.create();
  await env.addOrigin();
  const slot = env.runGet();
  const child = spawn("sleep", ["30"], { cwd: slot, stdio: "ignore" });
  try {
    expect(env.statusWord(slot)).toBe("available");
    const slot2 = env.runGet();
    expect(slot2, "slot with a process in it should be reused").toBe(slot);
  } finally {
    child.kill();
  }
});
