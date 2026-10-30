/**
 * E2E tests for `bs get` (and the default-command behaviour).
 *
 * Every test uses `GitEnv`, which spins up a Docker container with an
 * `alpine/git` image. All git operations (init, commit, worktree
 * manipulation) run inside that container so the host machine's git
 * configuration, environment variables, and `~/.bonsai` directory are never
 * touched.
 */
import { existsSync, mkdtempSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, it } from "vitest";
import {
  branchFromOutput,
  currentBranch,
  expectFail,
  expectOk,
  GitEnv,
  hostGit,
  pathFromOutput,
  worktreeHead,
} from "./support";

// ── repo_slug uses main repo, not the slot name ───────────────────────────────

it("slug derived from main repo not slot", async () => {
  const env = await GitEnv.create();
  const slot1 = env.runGet();

  // Dirty slot1 so the next call is forced to create a new slot.
  writeFileSync(join(slot1, "dirty.txt"), "dirty");

  // Run `bs get` from *inside* slot1 (a linked worktree).
  const out = env.bsFrom(slot1, ["get"]);
  expectOk(out, "bs get from linked worktree");

  const slot2 = pathFromOutput(out.stdout);
  const slugDir = env.slugDir();

  // The new slot sits under the *main repo's* slug directory.
  expect(
    slot2.startsWith(slugDir),
    `slot from linked worktree should be under ${slugDir}, got ${slot2}`,
  ).toBe(true);
});

// ── dirty slot skipped → new slot created ─────────────────────────────────────

it("dirty slot skipped, new slot created", async () => {
  const env = await GitEnv.create();
  const slot1 = env.runGet();

  // Untracked file makes `git status --porcelain` non-empty → slot is dirty.
  writeFileSync(join(slot1, "dirty.txt"), "dirty");

  const slot2 = env.runGet();
  expect(slot2, "should return a fresh slot").not.toBe(slot1);
  expect(existsSync(slot2), "new slot must exist on disk").toBe(true);
  expect(existsSync(slot1), "dirty slot must not be removed").toBe(true);
});

// ── locked slot skipped → new slot created ────────────────────────────────────

it("locked slot skipped, new slot created", async () => {
  const env = await GitEnv.create();
  const slot1 = env.runGet();

  // Lock via host git (worktree is registered in the host repo).
  hostGit(env.repoPath, ["worktree", "lock", slot1]);

  const slot2 = env.runGet();
  expect(slot2, "should skip the locked slot").not.toBe(slot1);
  expect(existsSync(slot2)).toBe(true);

  // Unlock for clean teardown.
  hostGit(env.repoPath, ["worktree", "unlock", slot1]);
});

// ── first clean unlocked slot is returned ─────────────────────────────────────

it("first clean unlocked slot returned", async () => {
  const env = await GitEnv.create();

  // slot1 → dirty
  const slot1 = env.runGet();
  writeFileSync(join(slot1, "dirty.txt"), "dirty");

  // slot2 → clean
  const slot2 = env.runGet();
  expect(slot1).not.toBe(slot2);

  // Third call: slot1 dirty, slot2 clean → slot2 reused.
  const slot3 = env.runGet();
  expect(slot3, "clean slot2 should be reused").toBe(slot2);
});

// ── empty pool → new UUID slot created ────────────────────────────────────────

it("empty pool creates new slot", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  expect(existsSync(slot), "slot must exist on disk").toBe(true);

  const name = slot.split("/").pop() as string;
  expect(name.length, `slot name must be 8 chars, got: ${name}`).toBe(8);
  expect(name, `slot name must be hex, got: ${name}`).toMatch(/^[0-9a-fA-F]+$/);
});

// ── pool dirs created on first run ────────────────────────────────────────────

it("pool dirs created on first run", async () => {
  const env = await GitEnv.create();

  expect(
    readdirSync(env.bonsaiPath),
    "bonsaiPath should be empty before first run",
  ).toEqual([]);

  const slot = env.runGet();
  expect(existsSync(slot)).toBe(true);
  expect(existsSync(env.slugDir())).toBe(true);
});

// ── pool dir creation is idempotent ───────────────────────────────────────────

it("pool dirs idempotent", async () => {
  const env = await GitEnv.create();
  env.runGet();
  // Second call must not error even though the directories already exist.
  env.runGet();
});

// ── existing clean slot is reused ─────────────────────────────────────────────

it("existing clean slot reused", async () => {
  const env = await GitEnv.create();
  const slot1 = env.runGet();
  const slot2 = env.runGet();

  expect(slot2, "same slot should be returned on second call").toBe(slot1);
  expect(env.slots().length, "only one slot should exist").toBe(1);
});

// ── slot reset to current HEAD after a new commit ─────────────────────────────

it("slot reset to new HEAD", async () => {
  const env = await GitEnv.create();

  const slot = env.runGet();
  const oldHead = env.headSha();
  expect(worktreeHead(slot)).toBe(oldHead);

  // Advance HEAD via the container (isolated from host git config).
  await env.makeCommit("second.txt", "v2");
  const newHead = env.headSha();
  expect(oldHead).not.toBe(newHead);

  // Second `bs get` should reset the existing clean slot to the new HEAD.
  const slot2 = env.runGet();
  expect(slot2, "same slot should be reused").toBe(slot);
  expect(
    worktreeHead(slot),
    "slot HEAD should be updated to the new commit",
  ).toBe(newHead);
});

// ── stale registration pruned ─────────────────────────────────────────────────

it("stale registration pruned", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  expect(existsSync(slot)).toBe(true);

  // Manually delete the slot directory to create a stale registration.
  const { rmSync } = await import("node:fs");
  rmSync(slot, { recursive: true, force: true });
  expect(existsSync(slot)).toBe(false);

  // Next `bs get` must prune the stale entry and create a fresh slot.
  const newSlot = env.runGet();
  expect(
    existsSync(newSlot),
    "a fresh slot should be created after pruning",
  ).toBe(true);
});

// ── called from a linked worktree ─────────────────────────────────────────────

it("called from linked worktree uses main repo slug", async () => {
  const env = await GitEnv.create();

  const slot1 = env.runGet();
  const slugDir = env.slugDir();

  // Dirty slot1 so the next call creates slot2.
  writeFileSync(join(slot1, "linked_test.txt"), "x");

  const out = env.bsFrom(slot1, ["get"]);
  expectOk(out, "bs get from linked worktree");

  const slot2 = pathFromOutput(out.stdout);
  expect(
    slot2.startsWith(slugDir),
    `slot2 should be under ${slugDir}, got ${slot2}`,
  ).toBe(true);
});

// ── `bs` with no subcommand behaves like `bs get` ─────────────────────────────

it("no subcommand behaves like bs get", async () => {
  const env = await GitEnv.create();

  const outNoarg = env.bs();
  expect(outNoarg.status, "bs with no args should exit 0").toBe(0);
  const pathNoarg = pathFromOutput(outNoarg.stdout);

  // The slot from the first call is now clean; `bs get` reuses it.
  const outGet = env.bs(["get"]);
  expect(outGet.status).toBe(0);
  const pathGet = pathFromOutput(outGet.stdout);

  expect(pathNoarg, "`bs` and `bs get` should return the same slot").toBe(
    pathGet,
  );
});

// ── emoji in stdout ───────────────────────────────────────────────────────────

it("get output contains tree emoji", async () => {
  const env = await GitEnv.create();
  const out = env.bs(["get"]);
  expect(out.stdout, "stdout should contain the 🌳 emoji").toContain("🌳");
});

// ── -b: creates new branch in fresh slot ──────────────────────────────────────

it("get -b creates new branch", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "-b", "my-feature"]);
  expectOk(out, "bs get -b");

  const slot = pathFromOutput(out.stdout);
  expect(existsSync(slot), "slot must exist on disk").toBe(true);

  // Slot should be on the requested branch, not detached HEAD.
  expect(currentBranch(slot)).toBe("my-feature");

  // Branch name should appear in stdout.
  expect(branchFromOutput(out.stdout)).toBe("my-feature");
});

// ── -b: reused slot also ends up on the requested branch ──────────────────────

it("get -b reused slot gets branch", async () => {
  const env = await GitEnv.create();

  // Provision one slot so the pool is non-empty.
  const slot1 = env.runGet();

  // Reuse that slot with -b.
  const out = env.bs(["get", "-b", "reuse-branch"]);
  expectOk(out, "bs get -b on reused slot");

  const slot2 = pathFromOutput(out.stdout);
  expect(slot2, "existing clean slot should be reused").toBe(slot1);
  expect(currentBranch(slot2)).toBe("reuse-branch");
});

// ── -b: fails when branch already exists ──────────────────────────────────────

it("get -b fails if branch exists", async () => {
  const env = await GitEnv.create();

  // Create the branch in the repo (container exec, no checkout).
  await env.git("branch", "existing-branch");

  const out = env.bs(["get", "-b", "existing-branch"]);

  expectFail(out, "bs get -b existing");
  expect(out.stderr, "stderr should name the conflicting branch").toContain(
    "existing-branch",
  );
});

// ── -B: creates branch when it does not exist ─────────────────────────────────

it("get -B creates fresh branch", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "-B", "new-branch"]);
  expectOk(out, "bs get -B");

  const slot = pathFromOutput(out.stdout);
  expect(currentBranch(slot)).toBe("new-branch");
  expect(branchFromOutput(out.stdout)).toBe("new-branch");
});

// ── -B: resets existing branch to current HEAD ────────────────────────────────

it("get -B resets existing branch", async () => {
  const env = await GitEnv.create();

  // Create a branch at the initial commit (before we advance HEAD).
  await env.git("branch", "to-reset");

  // Advance HEAD so the branch is now behind.
  await env.makeCommit("second.txt", "v2");
  const newHead = env.headSha();

  const out = env.bs(["get", "-B", "to-reset"]);
  expectOk(out, "bs get -B reset");

  const slot = pathFromOutput(out.stdout);
  expect(currentBranch(slot)).toBe("to-reset");
  expect(worktreeHead(slot), "branch should be reset to the new HEAD").toBe(
    newHead,
  );
});

// ── no flags: slot stays in detached HEAD (regression guard) ──────────────────

it("get without flags is detached HEAD", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  // `git symbolic-ref HEAD` exits non-zero for detached HEAD.
  const out = hostGit(slot, ["symbolic-ref", "--short", "HEAD"]);
  expect(out.status, "slot should be in detached HEAD state").not.toBe(0);

  // pathFromOutput should still work (no branch suffix).
  const reOut = env.bs(["get"]);
  expect(branchFromOutput(reOut.stdout)).toBeNull();
});

// ── -b and -B together are rejected ───────────────────────────────────────────

it("get -b and -B together are rejected", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "-b", "foo", "-B", "bar"]);

  expectFail(out, "supplying both -b and -B");
  expect(
    out.stderr,
    "stderr should describe the mutual-exclusion constraint",
  ).toContain("cannot be used with");
});

/** Regression guard: the positional `<branch>` must not affect plain -b. */
it("get -b alone is unaffected by positional arg", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "-b", "solo-branch"]);
  expectOk(out, "bs get -b alone");
  const slot = pathFromOutput(out.stdout);
  expect(currentBranch(slot)).toBe("solo-branch");
});

// ── positional branch: checks out an existing, unclaimed branch ───────────────

it("get positional branch checks out existing branch", async () => {
  const env = await GitEnv.create();

  // Create the branch in the repo without checking it out anywhere.
  await env.git("branch", "my-existing-branch");

  const out = env.bs(["get", "my-existing-branch"]);
  expectOk(out, "bs get <branch>");

  const slot = pathFromOutput(out.stdout);
  expect(existsSync(slot), "slot must exist on disk").toBe(true);
  expect(
    currentBranch(slot),
    "slot should be checked out on the existing branch, not detached",
  ).toBe("my-existing-branch");

  expect(out.stdout, "stdout should contain the 🌳 emoji").toContain("🌳");
  expect(branchFromOutput(out.stdout)).toBe("my-existing-branch");
});

// ── positional branch: non-existent branch errors out ─────────────────────────

it("get positional branch nonexistent errors", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "no-such-branch"]);

  expectFail(out, "bs get <branch> for a missing branch");
  expect(out.stderr, "stderr should name the missing branch").toContain(
    "no-such-branch",
  );

  // No slot should have been created or reset: any slug directory created
  // under the pool root must contain no slot subdirectories.
  for (const slug of readdirSync(env.bonsaiPath)) {
    const slots = readdirSync(join(env.bonsaiPath, slug));
    expect(
      slots,
      "no worktree slot should be created for a non-existent branch",
    ).toEqual([]);
  }
});

// ── positional branch: already checked out in another managed slot ────────────

/**
 * When `<branch>` is already checked out in another bonsai-managed pool slot
 * for this repo, `bs get <branch>` returns that existing slot's path (with
 * branch name) and exits 0, without provisioning or resetting a slot.
 */
it("get positional branch already checked out in managed slot returns existing slot", async () => {
  const env = await GitEnv.create();

  // Provision a slot and check out `shared-branch` in it via -b.
  const out = env.bs(["get", "-b", "shared-branch"]);
  expect(out.status, "initial bs get -b should succeed").toBe(0);
  const claimedSlot = pathFromOutput(out.stdout);

  // Dirty the claimed slot so slot selection would skip it if the
  // branch-ownership pre-check were not in place.
  writeFileSync(join(claimedSlot, "dirty.txt"), "dirty");

  const out2 = env.bs(["get", "shared-branch"]);
  expectOk(out2, "bs get <branch> on an already-claimed branch");

  expect(
    pathFromOutput(out2.stdout),
    "should return the existing slot that already has shared-branch",
  ).toBe(claimedSlot);
  expect(branchFromOutput(out2.stdout)).toBe("shared-branch");

  // The dirtying marker must still be present: the slot was not reset.
  expect(
    existsSync(join(claimedSlot, "dirty.txt")),
    "the existing slot must not be reset when returned via the pre-check",
  ).toBe(true);

  // No second slot should have been created for this repo's pool.
  expect(
    readdirSync(env.slugDir()).length,
    "no new slot should be created when the branch is already claimed",
  ).toBe(1);
});

/**
 * When the already-claimed managed slot has additionally been locked via
 * `bs lock`, `bs get <branch>` must still return that slot's path and the
 * slot must remain locked afterward.
 */
it("get positional branch already checked out in locked managed slot returns existing slot", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "-b", "shared-branch"]);
  expect(out.status, "initial bs get -b should succeed").toBe(0);
  const claimedSlot = pathFromOutput(out.stdout);

  expectOk(env.bs(["lock", claimedSlot]), "bs lock");

  const out2 = env.bs(["get", "shared-branch"]);
  expectOk(out2, "bs get <branch> with a locked existing slot");
  expect(
    pathFromOutput(out2.stdout),
    "should return the locked slot that already has shared-branch",
  ).toBe(claimedSlot);

  // The slot must remain locked afterward.
  const porcelain = hostGit(env.repoPath, ["worktree", "list", "--porcelain"]);
  expect(
    porcelain.stdout,
    "slot should remain locked after bs get <branch>",
  ).toContain("locked");
});

/**
 * Regression: calling `bs get shared-branch` twice in a row after the branch
 * was provisioned via `-b` must both succeed and return the same slot.
 */
it("get positional branch repeated calls return same slot", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "-b", "shared-branch"]);
  expect(out.status, "initial bs get -b should succeed").toBe(0);
  const claimedSlot = pathFromOutput(out.stdout);

  const out2 = env.bs(["get", "shared-branch"]);
  expectOk(out2, "first repeated bs get shared-branch");
  expect(pathFromOutput(out2.stdout)).toBe(claimedSlot);

  const out3 = env.bs(["get", "shared-branch"]);
  expectOk(out3, "second repeated bs get shared-branch");
  expect(pathFromOutput(out3.stdout)).toBe(claimedSlot);
});

// ── positional branch: already checked out in an unmanaged worktree ──────────

it("get positional branch already checked out in unmanaged worktree errors", async () => {
  const env = await GitEnv.create();

  // Create a worktree OUTSIDE the bonsai pool, on the host, on
  // `unmanaged-branch`.
  const unmanagedDir = mkdtempSync(join(tmpdir(), "bonsai-unmanaged-"));
  const unmanagedPath = join(unmanagedDir, "wt");
  const addOut = hostGit(env.repoPath, [
    "worktree",
    "add",
    "-b",
    "unmanaged-branch",
    unmanagedPath,
  ]);
  expect(
    addOut.status,
    `failed to create unmanaged worktree: ${addOut.stderr}`,
  ).toBe(0);

  const out = env.bs(["get", "unmanaged-branch"]);

  expectFail(
    out,
    "bs get <branch> for a branch checked out in an unmanaged worktree",
  );
  expect(
    out.stderr.includes(unmanagedPath) ||
      out.stderr.includes("unmanaged-branch"),
    `stderr should name the conflicting worktree path or branch; got: ${out.stderr}`,
  ).toBe(true);
});

// ── positional branch vs -b/-B: mutual exclusion ──────────────────────────────

it("get positional branch conflicts with -b", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "-b", "foo", "existing-branch"]);

  expectFail(out, "positional branch + -b");
  expect(
    out.stderr,
    "stderr should describe the mutual-exclusion constraint",
  ).toContain("cannot be used with");
});

it("get positional branch conflicts with -B", async () => {
  const env = await GitEnv.create();

  const out = env.bs(["get", "-B", "foo", "existing-branch"]);

  expectFail(out, "positional branch + -B");
  expect(
    out.stderr,
    "stderr should describe the mutual-exclusion constraint",
  ).toContain("cannot be used with");
});
