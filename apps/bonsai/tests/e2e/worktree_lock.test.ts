/**
 * E2E tests for `bs lock` and `bs unlock`.
 *
 * All tests use `GitEnv` to run in a fully isolated Docker-backed git
 * environment. Host `~/.bonsai` is never touched.
 */
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { expectOk, GitEnv, hostGit } from "./support";

function porcelain(env: GitEnv): string {
  return hostGit(env.repoPath, ["worktree", "list", "--porcelain"]).stdout;
}

/** `bs lock <path>` locks the slot. */
it("lock by explicit path marks slot locked", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  const out = env.bs(["lock", slot]);
  expectOk(out, "bs lock");
  expect(
    out.stdout,
    "bs lock should print a confirmation containing 'locked'",
  ).toContain("locked");

  expect(
    porcelain(env),
    "git worktree list --porcelain should show 'locked' after bs lock",
  ).toContain("locked");
});

/** Lock a slot then unlock it; the slot returns to `available`. */
it("unlock restores available status", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  expectOk(env.bs(["lock", slot]), "bs lock");

  const unlockOut = env.bs(["unlock", slot]);
  expectOk(unlockOut, "bs unlock");
  expect(unlockOut.stdout, "bs unlock should print 'unlocked'").toContain(
    "unlocked",
  );

  const statusOut = env.bs(["status", slot]);
  expect(statusOut.status).toBe(0);
  expect(statusOut.stdout, "slot should be 'available' after unlock").toContain(
    "available",
  );
  expect(
    statusOut.stdout,
    "slot should not show 'locked' after unlock",
  ).not.toContain("locked");
});

/** `bs lock --reason <msg>` forwards the reason to git. */
it("lock with reason stores reason", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  const reason = "reserved for agent build";
  expectOk(env.bs(["lock", "--reason", reason, slot]), "bs lock --reason");

  expect(
    porcelain(env),
    "git porcelain should include the reason string",
  ).toContain(reason);
});

/** `bs lock`/`bs unlock` with no path default to the slot containing the CWD. */
it("lock/unlock defaults to current slot", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  expectOk(env.bsFrom(slot, ["lock"]), "bs lock (no path)");
  expect(
    porcelain(env),
    "slot should be locked after default bs lock",
  ).toContain("locked");

  expectOk(env.bsFrom(slot, ["unlock"]), "bs unlock (no path)");

  // Run from `repoPath` (not from inside `slot`) so the `bs status`
  // process's own CWD doesn't sit at the slot root.
  const statusOut = env.bs(["status", slot]);
  expect(
    statusOut.stdout,
    "slot should be available after default bs unlock",
  ).toContain("available");
});

/**
 * A locked slot is reported as `locked` (not `in use`) by `bs list` and
 * `bs status`; locked + dirty stays `locked`.
 */
it("status shows locked for locked slot", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  expect(env.bs(["lock", slot]).status).toBe(0);

  const listOut = env.bs(["list"]);
  expect(listOut.status).toBe(0);
  expect(listOut.stdout, "bs list should show the 'locked' badge").toContain(
    "locked",
  );
  expect(
    listOut.stdout,
    "bs list must not show 'in use' for a locked slot",
  ).not.toContain("in use");

  const statusOut = env.bs(["status", slot]);
  expect(statusOut.status).toBe(0);
  expect(statusOut.stdout).toContain("locked");
  expect(statusOut.stdout).not.toContain("in use");

  // Now dirty the slot (locked + dirty should still show as `locked`).
  writeFileSync(join(slot, "dirty.txt"), "dirty");

  const statusOut2 = env.bs(["status", slot]);
  expect(statusOut2.status).toBe(0);
  expect(
    statusOut2.stdout,
    "locked+dirty slot should still show 'locked'",
  ).toContain("locked");
  expect(
    statusOut2.stdout,
    "locked+dirty slot must not show 'in use'",
  ).not.toContain("in use");
});
