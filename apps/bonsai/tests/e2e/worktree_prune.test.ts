/**
 * E2E tests for `bs prune`.
 *
 * All tests use `GitEnv` to run in a fully isolated Docker-backed git
 * environment. Host `~/.bonsai` is never touched.
 */
import { chmodSync, existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { expectOk, GitEnv, hostGit, tildePath } from "./support";

function lock(env: GitEnv, slot: string): void {
  expect(env.bs(["lock", slot]).status, `bs lock ${slot}`).toBe(0);
}

function unlock(env: GitEnv, slot: string): void {
  expect(env.bs(["unlock", slot]).status, `bs unlock ${slot}`).toBe(0);
}

/** `bs prune` with no pool directory prints a friendly message and exits 0. */
it("prune no pool prints friendly message", async () => {
  const env = await GitEnv.create();

  // No `bs get` has been run — pool dir does not exist yet.
  const out = env.bs(["prune"]);
  expectOk(out, "bs prune");
  expect(out.stdout, "expected a friendly 'No worktrees' message").toContain(
    "No worktrees",
  );
});

/** Only locked/in-use slots: nothing is deleted, "nothing to prune" is printed. */
it("prune only locked and in-use slots prunes nothing", async () => {
  const env = await GitEnv.create();

  const lockedSlot = env.runGet();
  lock(env, lockedSlot);

  const dirtySlot = env.runGet();
  writeFileSync(join(dirtySlot, "dirty.txt"), "dirty");

  const out = env.bs(["prune"]);
  expectOk(out, "bs prune");
  expect(out.stdout.toLowerCase()).toContain("nothing to prune");

  expect(existsSync(lockedSlot), "locked slot must remain on disk").toBe(true);
  expect(existsSync(dirtySlot), "in-use (dirty) slot must remain on disk").toBe(
    true,
  );
});

/**
 * A pool with a locked, an in-use, and two available slots: locked and in-use
 * slots are untouched; of the two available slots exactly one is preserved
 * and the other deleted (which one is an internal ordering detail).
 */
it("prune mixed pool preserves one available slot", async () => {
  const env = await GitEnv.create();

  const lockedSlot = env.runGet();
  lock(env, lockedSlot);

  const inUseSlot = env.runGet();
  writeFileSync(join(inUseSlot, "dirty.txt"), "dirty");

  // Two available slots, registered separately.
  const slotA = env.runGet();
  // Lock it temporarily so the next `bs get` creates a genuinely new slot.
  lock(env, slotA);
  const slotB = env.runGet();
  unlock(env, slotA);

  const out = env.bs(["prune"]);
  expectOk(out, "bs prune");

  expect(existsSync(lockedSlot), "locked slot must remain on disk").toBe(true);
  expect(existsSync(inUseSlot), "in-use slot must remain on disk").toBe(true);

  const aExists = existsSync(slotA);
  const bExists = existsSync(slotB);
  expect(
    aExists !== bExists,
    `exactly one available slot should be preserved (slotA exists: ${aExists}, slotB exists: ${bExists})`,
  ).toBe(true);
  const [preserved, deleted] = aExists ? [slotA, slotB] : [slotB, slotA];

  // The deleted slot is no longer a registered worktree.
  const listOut = env.bs(["list"]);
  expect(listOut.status).toBe(0);
  const list = listOut.stdout;

  expect(list, "locked slot should still be registered").toContain(
    tildePath(lockedSlot),
  );
  expect(list, "in-use slot should still be registered").toContain(
    tildePath(inUseSlot),
  );
  expect(list, "preserved slot should still be registered").toContain(
    tildePath(preserved),
  );
  expect(
    list,
    "deleted slot should no longer be registered after `git worktree prune`",
  ).not.toContain(tildePath(deleted));
});

/** Exactly one available slot (plus a locked one): it is preserved, not deleted. */
it("prune single available slot is preserved, not deleted", async () => {
  const env = await GitEnv.create();

  const lockedSlot = env.runGet();
  lock(env, lockedSlot);

  const availableSlot = env.runGet();

  const out = env.bs(["prune"]);
  expectOk(out, "bs prune");

  expect(existsSync(lockedSlot), "locked slot must remain on disk").toBe(true);
  expect(
    existsSync(availableSlot),
    "the sole available slot must be preserved, not deleted",
  ).toBe(true);

  expect(
    !out.stdout.includes("kept") &&
      !out.stdout.includes(tildePath(availableSlot)),
    `expected no output identifying the preserved slot, got: ${out.stdout}`,
  ).toBe(true);
  expect(
    out.stdout,
    "expected the sole-preserve run to report 'nothing to prune'",
  ).toContain("Nothing to prune");
});

/**
 * When the sole available slot has a branch checked out, `bs prune` preserves
 * its directory but detaches the branch so it stays immediately reusable.
 */
it("prune preserved slot with branch is detached", async () => {
  const env = await GitEnv.create();

  const availableSlot = env.runGet();
  const checkout = hostGit(availableSlot, ["checkout", "-b", "my-feature"]);
  expect(checkout.status, `git checkout -b failed: ${checkout.stderr}`).toBe(0);

  const out = env.bs(["prune"]);
  expectOk(out, "bs prune");

  expect(
    existsSync(availableSlot),
    "preserved slot's directory must not be deleted",
  ).toBe(true);

  // The branch should no longer be checked out in that slot.
  const branchOut = hostGit(availableSlot, ["branch", "--show-current"]);
  expect(branchOut.status).toBe(0);
  expect(
    branchOut.stdout.trim(),
    `preserved slot should be in detached HEAD, got branch: ${branchOut.stdout}`,
  ).toBe("");

  expect(
    !out.stdout.includes("kept") &&
      !out.stdout.includes(tildePath(availableSlot)),
    `expected no output identifying the preserved slot, got: ${out.stdout}`,
  ).toBe(true);
  expect(
    out.stdout,
    "expected no mention of the detached branch",
  ).not.toContain("my-feature");
  expect(out.stdout).toContain("Nothing to prune");
});

/** Sole available slot already detached: reported without mentioning a branch. */
it("prune preserved slot already detached reports no branch", async () => {
  const env = await GitEnv.create();

  const availableSlot = env.runGet();

  const out = env.bs(["prune"]);
  expectOk(out, "bs prune");

  expect(existsSync(availableSlot)).toBe(true);
  expect(
    !out.stdout.includes("kept") &&
      !out.stdout.includes(tildePath(availableSlot)),
    `expected no output identifying the preserved slot, got: ${out.stdout}`,
  ).toBe(true);
  expect(out.stdout).toContain("Nothing to prune");
});

/**
 * A deleted slot with a checked-out branch is reported with the branch name;
 * the preserved slot (whichever is picked) produces no output at all.
 */
it("prune reports branch or no-branch suffix", async () => {
  const env = await GitEnv.create();

  const detachedSlot = env.runGet();
  lock(env, detachedSlot);

  const branchSlot = env.runGet();
  const checkout = hostGit(branchSlot, ["checkout", "-b", "my-feature"]);
  expect(checkout.status, `git checkout -b failed: ${checkout.stderr}`).toBe(0);

  unlock(env, detachedSlot);

  const out = env.bs(["prune"]);
  expectOk(out, "bs prune");
  const stdout = out.stdout;

  const branchTilde = tildePath(branchSlot);
  const detachedTilde = tildePath(detachedSlot);

  // Exactly one of the two slots is preserved (silently); the other deleted.
  expect(
    existsSync(branchSlot) !== existsSync(detachedSlot),
    "exactly one of the two available slots should remain",
  ).toBe(true);

  const lines = stdout.split("\n");
  if (existsSync(branchSlot)) {
    expect(
      stdout,
      "expected no output identifying the preserved branch slot",
    ).not.toContain(branchTilde);
    const detachedLine = lines.find((l) => l.includes(detachedTilde));
    expect(
      detachedLine,
      `expected a line for the deleted detached slot, got: ${stdout}`,
    ).toBeDefined();
    expect(detachedLine).toContain("pruned");
  } else {
    const branchLine = lines.find((l) => l.includes(branchTilde));
    expect(
      branchLine,
      `expected a line for the deleted branch slot, got: ${stdout}`,
    ).toBeDefined();
    expect(branchLine).toContain("pruned");
    expect(branchLine).toContain("my-feature");
    expect(
      stdout,
      "expected no output identifying the preserved detached-HEAD slot",
    ).not.toContain(detachedTilde);
  }
});

/**
 * A per-slot deletion failure (simulated by revoking write permission on the
 * slot's directory) is reported and does not prevent the preserved slot or
 * the other deletable slot from being handled, nor `git worktree prune` from
 * running; the process exits non-zero.
 *
 * Best-effort: skipped when running as root, since permissions have no
 * effect there.
 */
it.skipIf(process.getuid?.() === 0)(
  "prune partial failure reports and continues",
  async () => {
    const env = await GitEnv.create();

    // Three available slots. Which one `bs prune` preserves is an internal
    // ordering detail, so use `bs list`'s reported order (which prune shares)
    // and chmod one of the two slots that is *not* first in that order.
    const slotA = env.runGet();
    lock(env, slotA);
    const slotB = env.runGet();
    lock(env, slotB);
    const slotC = env.runGet();
    unlock(env, slotA);
    unlock(env, slotB);

    const listOut = env.bs(["list"]);
    expect(listOut.status).toBe(0);
    const list = listOut.stdout;

    const pos = (slot: string): number => {
      const idx = list.indexOf(tildePath(slot));
      expect(
        idx,
        `expected ${slot} in \`bs list\` output: ${list}`,
      ).toBeGreaterThanOrEqual(0);
      return idx;
    };
    const slots = [slotA, slotB, slotC].sort((x, y) => pos(x) - pos(y));

    const preservedSlot = slots[0] as string;
    const undeletableSlot = slots[1] as string;
    const deletableSlot = slots[2] as string;

    // Revoke write+execute on the slot itself so its contents can't be removed.
    chmodSync(undeletableSlot, 0o500);

    const out = env.bs(["prune"]);

    // Restore permissions unconditionally so temp dir cleanup can proceed.
    chmodSync(undeletableSlot, 0o700);

    if (out.status === 0) {
      // Permissions had no effect (privileged runner) — nothing to assert.
      return;
    }

    expect(
      existsSync(preservedSlot),
      "the preserved (first-ordered) available slot should remain untouched",
    ).toBe(true);
    expect(
      existsSync(undeletableSlot),
      "the chmod'd slot's deletion should have failed, leaving it on disk",
    ).toBe(true);
    expect(
      existsSync(deletableSlot),
      "the other deletable available slot should still be pruned",
    ).toBe(false);
    expect(
      out.stdout.includes(tildePath(deletableSlot)) ||
        out.stderr.includes(tildePath(deletableSlot)),
      `expected the successfully pruned slot to be reported, stdout: ${out.stdout}`,
    ).toBe(true);
    expect(
      out.stderr.includes(tildePath(undeletableSlot)) ||
        out.stdout.includes(tildePath(undeletableSlot)),
      `expected the failed slot to be reported, stderr: ${out.stderr}`,
    ).toBe(true);
  },
);

/** Force with no paths: locked slot untouched, dirty preserved slot sanitized, branch survives. */
it("prune --force sanitizes the preserved slot and keeps branches", async () => {
  const env = await GitEnv.create();

  const lockedSlot = env.runGet();
  writeFileSync(join(lockedSlot, "keep.txt"), "mine");
  lock(env, lockedSlot);

  await env.git("branch", "my-feature");
  const slot = env.runGetBranch("my-feature");
  writeFileSync(join(slot, "dirty.txt"), "dirty");
  expect(env.statusWord(slot)).toBe("in use");

  const out = env.bs(["prune", "--force"]);
  expectOk(out, "bs prune --force");

  expect(existsSync(slot), "preserved slot stays on disk").toBe(true);
  expect(existsSync(join(slot, "dirty.txt"))).toBe(false);
  expect(env.statusWord(slot)).toBe("available");
  expect(existsSync(join(lockedSlot, "keep.txt"))).toBe(true);
  expect(
    hostGit(env.repoPath, ["branch", "--list", "my-feature"]).stdout,
  ).toContain("my-feature");
});

/** Locked targets are refused with or without --force; unlock makes them prunable. */
it("prune refuses locked paths until unlocked", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();
  lock(env, slot);

  for (const args of [
    ["prune", slot],
    ["prune", "--force", slot],
  ]) {
    const out = env.bs(args);
    expect(out.status).not.toBe(0);
    expect(out.stderr).toContain("unlock");
    expect(existsSync(slot)).toBe(true);
  }

  unlock(env, slot);
  expectOk(env.bs(["prune", "--force", slot]), "bs prune --force after unlock");
  expect(existsSync(slot)).toBe(true); // lone slot is preserved
});

/** An in-use path requires --force; with an available sibling it is deleted. */
it("prune in-use path needs --force", async () => {
  const env = await GitEnv.create();
  const busy = env.runGet();
  writeFileSync(join(busy, "dirty.txt"), "dirty");
  lock(env, busy);
  const free = env.runGet();
  unlock(env, busy);

  const refused = env.bs(["prune", busy]);
  expect(refused.status).not.toBe(0);
  expect(refused.stderr).toContain("--force");
  expect(existsSync(busy)).toBe(true);

  expectOk(env.bs(["prune", "--force", busy]), "bs prune --force <path>");
  expect(existsSync(busy)).toBe(false);
  expect(existsSync(free)).toBe(true);
});
