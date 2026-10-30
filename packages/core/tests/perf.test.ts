/**
 * Performance SLO for `bs list`'s per-slot classification path
 * (`listWorktreesStatus`), enforcing the thresholds in the `worktree-list`
 * spec. Runs against a fake git port that simulates per-process cost (with the
 * same bounded process parallelism as the real adapter), so it measures the
 * algorithmic fan-out of the classification path, not the host machine's git.
 *
 * Per-slot short-circuiting is what keeps these scenarios cheap:
 *   1. all locked        → no per-slot git call
 *   2. all dirty         → one `git status` per slot
 *   3. all available     → `git status` + branch-state probes per slot
 *
 * Changing a threshold requires a spec update and user approval.
 */
import { describe, expect, it } from "vitest";
import { listWorktreesStatus, type WorktreeStatus } from "../src/index.ts";
import { addSlots, makeWorld, POOL } from "./fakes.ts";

/** SLO thresholds from openspec/specs/worktree-list/spec.md. */
const SLO = {
  /** p95 latency at 50 slots (locked and dirty scenarios), ms. */
  absoluteP95Ms50: 100,
  /** p95(50 slots) / p95(5 slots) for the all-locked scenario. */
  lockedScalingRatio: 6.0,
  /** p95(50 slots) / p95(5 slots) for the dirty and available scenarios. */
  linearScalingRatio: 15,
} as const;

/** Simulated cost of one git process, ms. */
const GIT_LIST_MS = 3;
const GIT_PROBE_MS = 3;
/** Same bound as the real git adapter's default. */
const MAX_PROCESSES = 16;

const SAMPLES = 30;
/** Latency floor (ms) so timer noise can't inflate near-zero ratios. */
const RATIO_FLOOR_MS = 1;

type Scenario = "locked" | "dirty" | "available";

async function runOnce(
  scenario: Scenario,
  slots: number,
): Promise<{ ms: number; statuses: WorktreeStatus[] }> {
  const w = makeWorld();
  w.git.listDelayMs = GIT_LIST_MS;
  w.git.probeDelayMs = GIT_PROBE_MS;
  w.git.maxConcurrency = MAX_PROCESSES;
  w.git.refs.add("refs/heads/main");

  const paths = addSlots(
    w,
    Array.from({ length: slots }, () => ({ locked: scenario === "locked" })),
  );
  for (const p of paths) {
    if (scenario === "dirty") {
      w.git.statuses.set(p, {
        uncommitted: [],
        untracked: [{ code: "??", path: "dirty.txt" }],
      });
    }
    if (scenario === "available") w.git.ancestorOf.set(p, true);
  }

  const start = performance.now();
  const entries = await listWorktreesStatus(w.deps, POOL);
  const ms = performance.now() - start;
  return { ms, statuses: entries.map((e) => e.status) };
}

async function p95(scenario: Scenario, slots: number): Promise<number> {
  const times: number[] = [];
  for (let i = 0; i < SAMPLES; i++)
    times.push((await runOnce(scenario, slots)).ms);
  times.sort((a, b) => a - b);
  return times[Math.ceil(SAMPLES * 0.95) - 1] as number;
}

const EXPECTED: Record<Scenario, WorktreeStatus> = {
  locked: "locked",
  dirty: "in use",
  available: "available",
};

/** Measure both pool sizes and return a one-line report. */
async function measure(scenario: Scenario) {
  const p5 = await p95(scenario, 5);
  const p50 = await p95(scenario, 50);
  const ratio = p50 / Math.max(p5, RATIO_FLOOR_MS);
  console.log(
    `perf [${scenario}] p95@5=${p5.toFixed(1)}ms p95@50=${p50.toFixed(1)}ms ratio=${ratio.toFixed(2)}x`,
  );
  return { p5, p50, ratio };
}

describe("bs list classification SLO", () => {
  it.each(["locked", "dirty", "available"] as const)(
    "%s scenario classifies every slot correctly at 5 and 50 slots",
    async (scenario) => {
      for (const n of [5, 50]) {
        const { statuses } = await runOnce(scenario, n);
        expect(statuses).toHaveLength(n);
        expect(new Set(statuses)).toEqual(new Set([EXPECTED[scenario]]));
      }
    },
  );

  it("all-locked: p95@50 <= 100ms and scaling <= 6x", async () => {
    const { p50, ratio } = await measure("locked");
    expect(p50, "p95 at 50 locked slots (ms)").toBeLessThanOrEqual(
      SLO.absoluteP95Ms50,
    );
    expect(ratio, "p95(50)/p95(5) for locked slots").toBeLessThanOrEqual(
      SLO.lockedScalingRatio,
    );
  });

  it("all-dirty: p95@50 <= 100ms and scaling <= 15x", async () => {
    const { p50, ratio } = await measure("dirty");
    expect(p50, "p95 at 50 dirty slots (ms)").toBeLessThanOrEqual(
      SLO.absoluteP95Ms50,
    );
    expect(ratio, "p95(50)/p95(5) for dirty slots").toBeLessThanOrEqual(
      SLO.linearScalingRatio,
    );
  });

  it("all-available: reported, gated only by the 15x scaling bound", async () => {
    const { ratio } = await measure("available");
    expect(ratio, "p95(50)/p95(5) for available slots").toBeLessThanOrEqual(
      SLO.linearScalingRatio,
    );
  });

  it("locked slots never fan out to per-slot git calls", async () => {
    const w = makeWorld();
    addSlots(
      w,
      Array.from({ length: 50 }, () => ({ locked: true })),
    );
    await listWorktreesStatus(w.deps, POOL);
    expect(w.git.calls).toEqual(["listWorktrees"]);
  });

  it("dirty slots never trigger branch-state probes", async () => {
    const w = makeWorld();
    const paths = addSlots(
      w,
      Array.from({ length: 50 }, () => ({})),
    );
    for (const p of paths) {
      w.git.statuses.set(p, {
        uncommitted: [{ code: " M", path: "a" }],
        untracked: [],
      });
    }
    await listWorktreesStatus(w.deps, POOL);
    expect(w.git.count("status")).toBe(50);
    expect(w.git.count("isAncestor")).toBe(0);
    expect(w.git.count("symbolicRefShort")).toBe(0);
  });

  it("the default ref is resolved once for the whole pool", async () => {
    const w = makeWorld();
    const paths = addSlots(
      w,
      Array.from({ length: 50 }, () => ({})),
    );
    w.git.refs.add("refs/heads/main");
    for (const p of paths) w.git.ancestorOf.set(p, true);
    await listWorktreesStatus(w.deps, POOL);
    expect(w.git.count("symbolicRefShort")).toBe(1);
  });
});
