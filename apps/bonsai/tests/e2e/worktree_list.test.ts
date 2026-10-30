/**
 * E2E tests for `bs list` / `bs ls`.
 *
 * All tests use `GitEnv` to run in a fully isolated Docker-backed git
 * environment. Host `~/.bonsai` is never touched.
 */
import { writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { expect, it } from "vitest";
import {
  expectOk,
  GitEnv,
  gitOnlyPathDir,
  hostGit,
  stripAnsi,
  tildePath,
} from "./support";

const NO_STAT_ICONS = (stdout: string) =>
  !stdout.includes("?1") &&
  !stdout.includes("\u00b1") &&
  !stdout.includes("\u2699");

// ── empty pool ────────────────────────────────────────────────────────────────

/** `bs list` with no pool directory prints a friendly message and exits 0. */
it("list with no pool prints friendly message", async () => {
  const env = await GitEnv.create();

  // No `bs get` has been run — pool dir does not exist yet.
  const out = env.bs(["list"]);
  expectOk(out, "bs list");
  expect(out.stdout, "expected a friendly 'No worktrees' message").toContain(
    "No worktrees",
  );
});

/** `bs ls` alias behaves identically to `bs list`. */
it("ls alias behaves like list", async () => {
  const env = await GitEnv.create();

  const listOut = env.bs(["list"]);
  const lsOut = env.bs(["ls"]);

  expect(listOut.status).toBe(0);
  expect(lsOut.status).toBe(0);
  expect(
    lsOut.stdout,
    "`bs list` and `bs ls` should produce identical output",
  ).toBe(listOut.stdout);
});

// ── one slot: path + badge, no stats column ───────────────────────────────────

it("list one slot shows path and badge", async () => {
  const env = await GitEnv.create();

  const slot = env.runGet();

  const out = env.bs(["list"]);
  expectOk(out, "bs list");

  expect(
    out.stdout,
    "bs list must print a status badge for a clean unlocked slot",
  ).toContain("available");
  expect(out.stdout, "output should contain the slot directory name").toContain(
    basename(slot),
  );
});

// ── branch display, no stats column ───────────────────────────────────────────

/**
 * After `bs get` provisions a slot in detached HEAD, attaching a branch and
 * adding an untracked file shows the branch name and the `in use` badge, but
 * never a stats column.
 */
it("list shows branch and badge but no stats column", async () => {
  const env = await GitEnv.create();
  const slot = env.runGet();

  const attach = hostGit(slot, ["checkout", "-b", "feature/my-work"]);
  expect(attach.status, `git checkout -b failed: ${attach.stderr}`).toBe(0);

  // An untracked file must NOT show up as a stats icon.
  writeFileSync(join(slot, "untracked.txt"), "hello");

  const out = env.bs(["list"]);
  expectOk(out, "bs list");

  expect(out.stdout).toContain("feature/my-work");
  expect(
    out.stdout,
    "an untracked file should classify the slot as 'in use'",
  ).toContain("in use");
  expect(
    NO_STAT_ICONS(out.stdout),
    `bs list must never show stat icons, got: ${out.stdout}`,
  ).toBe(true);
});

/** A dirty, unlocked slot is displayed as `in use`. */
it("list dirty slot shows in-use badge, no stats", async () => {
  const env = await GitEnv.create();

  const slot = env.runGet();
  writeFileSync(join(slot, "dirty.txt"), "dirty");

  const out = env.bs(["list"]);
  expectOk(out, "bs list for a dirty slot");

  expect(out.stdout).toContain("in use");
  expect(
    NO_STAT_ICONS(out.stdout),
    `bs list must never show stat icons, got: ${out.stdout}`,
  ).toBe(true);
});

/** A locked slot is classified `locked`, taking priority over dirtiness. */
it("list locked slot shows locked badge", async () => {
  const env = await GitEnv.create();

  const slot = env.runGet();
  writeFileSync(join(slot, "dirty.txt"), "dirty");

  expect(env.bs(["lock", slot]).status).toBe(0);

  const out = env.bs(["list"]);
  expect(out.status).toBe(0);
  expect(out.stdout).toContain("locked");
  expect(out.stdout, "a locked slot must not also show 'in use'").not.toContain(
    "in use",
  );
});

// ── column alignment across badges of different lengths ───────────────────────

/** Column at which `needle` starts on any (ANSI-stripped) line of `stdout`. */
function columnOf(stdout: string, needle: string): number | undefined {
  let col: number | undefined;
  for (const line of stdout.split("\n")) {
    const plain = stripAnsi(line);
    const idx = plain.indexOf(needle);
    if (idx >= 0) col = [...plain.slice(0, idx)].length;
  }
  return col;
}

/**
 * The worktree path column must start at the same character position on every
 * line, regardless of badge length.
 */
it("list path column aligned across badge lengths", async () => {
  const env = await GitEnv.create();

  const availableSlot = env.runGet();
  const lockedSlot = env.runGet();

  expect(env.bs(["lock", lockedSlot]).status).toBe(0);

  const out = env.bs(["list"]);
  expect(out.status).toBe(0);

  const availableCol = columnOf(out.stdout, tildePath(availableSlot));
  const lockedCol = columnOf(out.stdout, tildePath(lockedSlot));

  expect(
    availableCol,
    "expected to find the 'available' slot's path in the output",
  ).toBeDefined();
  expect(
    lockedCol,
    "expected to find the 'locked' slot's path in the output",
  ).toBeDefined();
  expect(
    availableCol,
    `path column should start at the same position regardless of badge length, got: ${out.stdout}`,
  ).toBe(lockedCol);
});

/** Alignment must hold even when the `▶` current-slot prefix is present. */
it("list path column aligned with current slot prefix", async () => {
  const env = await GitEnv.create();

  const availableSlot = env.runGet();
  const lockedSlot = env.runGet();

  expect(env.bs(["lock", lockedSlot]).status).toBe(0);

  // Run `bs list` from inside the locked slot so its row gets the `▶` prefix.
  const out = env.bsFrom(lockedSlot, ["list"]);
  expect(out.status).toBe(0);

  expect(out.stdout, "expected the current-slot marker in output").toContain(
    "▶",
  );

  const availableCol = columnOf(out.stdout, tildePath(availableSlot));
  const lockedCol = columnOf(out.stdout, tildePath(lockedSlot));

  expect(availableCol).toBeDefined();
  expect(lockedCol).toBeDefined();
  expect(
    availableCol,
    `path column should be aligned with the ▶ prefix present, got: ${out.stdout}`,
  ).toBe(lockedCol);
});

// ── no external tools beyond git ──────────────────────────────────────────────

/** `bs list` needs only `git`: with a PATH exposing nothing else it must succeed. */
it("list needs only git", async () => {
  const env = await GitEnv.create();
  const locked = env.runGet();
  expect(env.bs(["lock", locked]).status).toBe(0);
  const dirty = env.runGet();
  writeFileSync(join(dirty, "dirty.txt"), "dirty");
  env.runGet(); // clean

  const out = env.bs(["list"], { env: { PATH: gitOnlyPathDir() } });

  expectOk(out, "bs list with only git on PATH");
  expect(out.stdout).toContain("locked");
  expect(out.stdout).toContain("in use");
  expect(out.stdout).toContain("available");
});

/** A clean slot whose branch has unmerged commits is `in use` in `bs list`. */
it("list clean slot with unmerged commits is in use", async () => {
  const env = await GitEnv.create();
  await env.git("checkout", "-q", "-b", "feat");
  await env.makeCommit("f.txt", "f");
  await env.git("checkout", "-q", "-");
  env.runGetBranch("feat");

  const out = env.bs(["list"]);
  expect(out.stdout).toContain("in use");
});
