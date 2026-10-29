/**
 * Managed git-worktree pool logic.
 *
 * Pool layout:
 *
 *     <managed_root>/
 *       <repo-slug>/          <- derived from the main repo root basename
 *         <8-hex-id>/         <- one slot per managed worktree
 */
import {
  type BranchState,
  branchState,
  isAvailable,
  LazyDefaultRef,
  reason,
  resolveDefaultRef,
} from "./branch-state.ts";
import { classify, slugify } from "./classify.ts";
import { BonsaiError, errorMessage } from "./errors.ts";
import * as path from "./path.ts";
import type { Deps, LogLocation } from "./ports.ts";
import type {
  BranchMode,
  PrunedSlot,
  PruneOutcome,
  SlotStatusReport,
  WorktreeEntry,
  WorktreeListEntry,
  WorktreeStatus,
} from "./types.ts";

const BARE_REPO_MESSAGE =
  "bare repositories are not supported by `bs get`; run from a non-bare working tree";

/** Canonicalise `p`, falling back to `p` itself when it cannot be resolved. */
async function canonicalOr(deps: Deps, p: string): Promise<string> {
  try {
    return await deps.fs.realpath(p);
  } catch {
    return p;
  }
}

/** Turn git's (possibly relative) common dir into an absolute path. */
function absoluteCommonDir(deps: Deps, raw: string): string {
  if (raw === ".") throw new BonsaiError(BARE_REPO_MESSAGE);
  return path.resolve(deps.env.cwd(), raw);
}

/** Repo slug (basename of the main repo root) for a git common dir. */
function slugFromCommonDir(commonDir: string): string {
  const repoRoot = path.dirname(commonDir);
  if (repoRoot === undefined) {
    throw new BonsaiError("`git common dir` path has no parent component");
  }
  const basename = path.basename(repoRoot);
  if (basename === undefined) {
    throw new BonsaiError("repo root directory has no usable file name");
  }
  return slugify(basename);
}

/** Basename of the **main** repo root, slugified; same from any worktree. */
export async function repoSlug(deps: Deps): Promise<string> {
  const raw = await deps.git.revParseCommonDir();
  return slugFromCommonDir(absoluteCommonDir(deps, raw));
}

/** `$BONSAI_ROOT` when set, otherwise `~/.bonsai`. */
export function managedRoot(deps: Deps): string {
  const root = deps.env.get("BONSAI_ROOT");
  if (root !== undefined) return root;
  const home = deps.env.homeDir();
  if (home === undefined) {
    throw new BonsaiError(
      "cannot resolve the home directory; please ensure the $HOME environment variable is set",
    );
  }
  return path.join(home, ".bonsai");
}

/** `~`-abbreviated path for display. */
export function tilde(deps: Deps, p: string): string {
  return path.tildePath(p, deps.env.homeDir());
}

async function poolDirFor(deps: Deps): Promise<string> {
  return path.join(managedRoot(deps), await repoSlug(deps));
}

// ── pool scan ──────────────────────────────────────────────────────────────

/**
 * Pool entries (canonicalised, under `poolDir`) plus a stale flag: `true`
 * when ANY registered worktree path no longer exists on disk, i.e. a
 * `git worktree prune` is warranted.
 */
async function listPoolWorktreesCheckingStale(
  deps: Deps,
  poolDir: string,
): Promise<{ entries: WorktreeEntry[]; hasStale: boolean }> {
  const raw = await deps.git.listWorktrees();
  // Canonicalise once so comparisons hold through symlinks (/tmp → /private/tmp).
  const poolCanonical = await canonicalOr(deps, poolDir);

  const entries: WorktreeEntry[] = [];
  let hasStale = false;
  for (const entry of raw) {
    const canonical = await canonicalOr(deps, entry.path);
    if (!(await deps.fs.exists(canonical))) hasStale = true;
    if (path.startsWith(canonical, poolCanonical)) {
      entries.push({ ...entry, path: canonical });
    }
  }
  return { entries, hasStale };
}

/** Registered worktrees whose path falls under `poolDir`. */
export async function listPoolWorktrees(
  deps: Deps,
  poolDir: string,
): Promise<WorktreeEntry[]> {
  return (await listPoolWorktreesCheckingStale(deps, poolDir)).entries;
}

/**
 * Classify one pool slot, short-circuiting as soon as the classification is
 * known so cheaper signals are checked first: locked (free) → dirty (one
 * `git status`) → branch state.
 */
export async function classifySlotStatus(
  deps: Deps,
  entry: WorktreeEntry,
  defaultRef: LazyDefaultRef,
): Promise<WorktreeStatus> {
  if (entry.locked) return classify(true, false, false);
  const status = await deps.git.status(entry.path);
  if (status.uncommitted.length > 0 || status.untracked.length > 0) {
    return classify(false, true, false);
  }
  const state = await branchState(
    deps.git,
    entry.path,
    entry.branch,
    await defaultRef.get(),
  );
  return classify(false, false, !isAvailable(state));
}

/**
 * Status of every pool slot. Per-slot checks run concurrently (the git
 * adapter bounds actual process parallelism); original order is preserved.
 */
export async function listWorktreesStatus(
  deps: Deps,
  poolDir: string,
): Promise<WorktreeListEntry[]> {
  const entries = await listPoolWorktrees(deps, poolDir);
  const defaultRef = new LazyDefaultRef(deps.git, ".");

  return Promise.all(
    entries.map(async (entry): Promise<WorktreeListEntry> => {
      const status = (await deps.fs.exists(entry.path))
        ? await classifySlotStatus(deps, entry, defaultRef)
        : "in use";
      return { path: entry.path, status, branch: entry.branch };
    }),
  );
}

/** `bs list` data. `undefined` when the pool does not exist yet. */
export async function listPool(
  deps: Deps,
): Promise<
  { entries: WorktreeListEntry[]; current: string | undefined } | undefined
> {
  const poolDir = await poolDirFor(deps);
  if (!(await deps.fs.exists(poolDir))) return undefined;

  const entries = await listWorktreesStatus(deps, poolDir);
  // Detect the current slot once; tolerate errors (treat as none).
  const current = await currentWorktree(deps).then(
    (c) => c?.path,
    () => undefined,
  );
  return { entries, current };
}

/** Detailed status report for a single registered pool slot. */
export async function slotStatus(
  deps: Deps,
  slot: string,
): Promise<SlotStatusReport> {
  let canonical: string;
  try {
    canonical = await deps.fs.realpath(slot);
  } catch (e) {
    throw new BonsaiError(
      `failed to canonicalize slot path ${slot}: ${errorMessage(e)}`,
    );
  }

  // Scoping the "pool" filter to the slot's own path means only this one
  // worktree entry (if any) passes the prefix check.
  const { entries } = await listPoolWorktreesCheckingStale(deps, canonical);
  const entry = entries.find((e) => e.path === canonical);
  if (entry === undefined) {
    throw new BonsaiError(`${canonical} is not a registered git worktree`);
  }

  const gitStatus = await deps.git.status(canonical);
  const dirty =
    gitStatus.uncommitted.length > 0 || gitStatus.untracked.length > 0;

  let hasUnmergedWork = false;
  let branchStateReason: string | undefined;
  if (!entry.locked) {
    const defaultRef = await resolveDefaultRef(deps.git, canonical);
    const state: BranchState = await branchState(
      deps.git,
      canonical,
      entry.branch,
      defaultRef,
    );
    hasUnmergedWork = !isAvailable(state);
    branchStateReason = reason(state, defaultRef);
  }

  return {
    path: canonical,
    branch: entry.branch,
    status: classify(entry.locked, dirty, hasUnmergedWork),
    lockReason: entry.lockReason,
    branchState: branchStateReason,
    gitStatus,
  };
}

// ── slot selection & provisioning ──────────────────────────────────────────

/** First available (clean, unlocked, on-disk, no unmerged work) slot. */
export async function findAvailableSlot(
  deps: Deps,
  poolDir: string,
): Promise<string | undefined> {
  const { entries, hasStale } = await listPoolWorktreesCheckingStale(
    deps,
    poolDir,
  );
  if (hasStale) {
    deps.log.info("Pruning stale worktrees from pool");
    await deps.git.worktreePrune();
  }
  const defaultRef = new LazyDefaultRef(deps.git, ".");
  for (const entry of entries) {
    if (entry.locked || !(await deps.fs.exists(entry.path))) continue;
    if ((await classifySlotStatus(deps, entry, defaultRef)) !== "available") {
      continue;
    }
    deps.log.info(`Found available slot: ${entry.path}`);
    return entry.path;
  }
  deps.log.debug("No available slots found in pool");
  return undefined;
}

function describeBranch(branch: BranchMode | undefined): string {
  if (branch === undefined) return "detached HEAD";
  const label = { new: "new", reset: "reset", existing: "existing" }[
    branch.kind
  ];
  return `${label} branch ${branch.name}`;
}

/** Reset an existing slot to `headSha`, optionally checking out a branch. */
export async function resetSlot(
  deps: Deps,
  slot: string,
  headSha: string,
  branch: BranchMode | undefined,
): Promise<void> {
  deps.log.debug(
    `Resetting slot ${slot} to ${headSha} (${describeBranch(branch)})`,
  );
  try {
    await deps.git.checkout(slot, headSha, branch);
  } catch (e) {
    deps.log.warn(`git checkout failed for slot ${slot}: ${errorMessage(e)}`);
    throw e;
  }
  deps.log.debug(`Successfully reset slot ${slot}`);
}

/** The first pool slot currently checked out on `branch`, if any. */
export async function findSlotCheckedOutOnBranch(
  deps: Deps,
  poolDir: string,
  branch: string,
): Promise<string | undefined> {
  if (!(await deps.fs.exists(poolDir))) return undefined;
  const entries = await listPoolWorktrees(deps, poolDir);
  return entries.find((e) => e.branch === branch)?.path;
}

/** Copy each relative `entries` path from `originRoot` into `slot`. */
async function copyIgnoredFiles(
  deps: Deps,
  originRoot: string,
  slot: string,
  entries: string[],
): Promise<void> {
  let copied = 0;
  let skipped = 0;
  for (const entry of entries) {
    const source = path.join(originRoot, entry);
    if (!(await deps.fs.exists(source))) {
      deps.log.debug(`bonsai.copy: skipping missing source file ${source}`);
      skipped++;
      continue;
    }
    const dest = path.join(slot, entry);
    const parent = path.dirname(dest);
    if (parent !== undefined && parent !== "") {
      try {
        await deps.fs.mkdirp(parent);
      } catch (e) {
        throw new BonsaiError(
          `failed to create destination directory ${parent}: ${errorMessage(e)}`,
        );
      }
    }
    try {
      await deps.fs.copyFile(source, dest);
    } catch (e) {
      throw new BonsaiError(
        `failed to copy ${source} to ${dest}: ${errorMessage(e)}`,
      );
    }
    copied++;
  }
  if (copied > 0 || skipped > 0) {
    deps.log.info(
      `bonsai.copy: copied ${copied} file(s), skipped ${skipped} missing entry(ies) into ${slot}`,
    );
  }
}

/**
 * Full `get` flow: provision (or reuse) a slot and return its canonical
 * path. With no `branch` the slot is left in detached HEAD.
 */
export async function getWorktree(
  deps: Deps,
  branch: BranchMode | undefined,
): Promise<string> {
  // The origin worktree: source root for `bonsai.copy` entries and the
  // directory `git config --get-all bonsai.copy` is resolved from.
  const originDir = deps.env.cwd();

  const { headSha, commonDir: rawCommonDir } =
    await deps.git.revParseHeadAndCommonDir();
  const slug = slugFromCommonDir(absoluteCommonDir(deps, rawCommonDir));
  let poolDir = path.join(managedRoot(deps), slug);

  try {
    await deps.fs.mkdirp(poolDir);
    poolDir = await deps.fs.realpath(poolDir);
  } catch (e) {
    throw new BonsaiError(
      `failed to create pool directory ${poolDir}: ${errorMessage(e)}`,
    );
  }

  // For a positional `<branch>`, first check whether one of this repo's
  // managed slots already has it checked out; if so hand that slot back
  // (git would refuse to check the branch out a second time).
  if (branch?.kind === "existing") {
    const existing = await findSlotCheckedOutOnBranch(
      deps,
      poolDir,
      branch.name,
    );
    if (existing !== undefined) return deps.fs.realpath(existing);
  }

  let slot = await findAvailableSlot(deps, poolDir);
  if (slot !== undefined) {
    await resetSlot(deps, slot, headSha, branch);
  } else {
    slot = path.join(poolDir, deps.env.randomSlotId());
    await deps.git.worktreeAdd(slot, headSha, branch);
  }

  // Final provisioning step: copy `bonsai.copy` files from the origin.
  const copyEntries = await deps.git.configGetAll(originDir, "bonsai.copy");
  if (copyEntries.length > 0) {
    await copyIgnoredFiles(deps, originDir, slot, copyEntries);
  }

  return deps.fs.realpath(slot);
}

// ── current slot ───────────────────────────────────────────────────────────

/** The managed slot containing the CWD, or `undefined`. */
export async function currentWorktree(
  deps: Deps,
): Promise<{ path: string; branch: string | undefined } | undefined> {
  let cwd: string;
  try {
    cwd = await deps.fs.realpath(deps.env.cwd());
  } catch (e) {
    throw new BonsaiError(
      `failed to canonicalize current directory: ${errorMessage(e)}`,
    );
  }
  const poolDir = await poolDirFor(deps);
  if (!(await deps.fs.exists(poolDir))) return undefined;
  const poolCanonical = await canonicalOr(deps, poolDir);
  const entries = await listPoolWorktrees(deps, poolCanonical);
  const hit = entries.find((e) => path.startsWith(cwd, e.path));
  return hit === undefined ? undefined : { path: hit.path, branch: hit.branch };
}

/** Resolve an explicit path argument or fall back to the current slot. */
export async function resolveTargetSlot(
  deps: Deps,
  explicit: string | undefined,
): Promise<string> {
  if (explicit !== undefined) return explicit;
  const current = await currentWorktree(deps);
  if (current === undefined) {
    throw new BonsaiError(
      "not inside a managed bonsai pool slot; please provide a path argument",
    );
  }
  return current.path;
}

/** Verify `slot` exists and lies under this repo's pool directory. */
export async function validatePoolSlot(
  deps: Deps,
  slot: string,
  poolDir: string,
): Promise<void> {
  if (!(await deps.fs.exists(slot))) {
    throw new BonsaiError(`path does not exist: ${slot}`);
  }
  const poolCanonical = await canonicalOr(deps, poolDir);
  const slotCanonical = await canonicalOr(deps, slot);
  if (!path.startsWith(slotCanonical, poolCanonical)) {
    throw new BonsaiError(
      `${slot} is not a bonsai-managed pool slot (pool directory: ${poolDir})`,
    );
  }
}

// ── status / lock / unlock use cases ───────────────────────────────────────

/** `bs status [PATH]` data. */
export async function statusForPath(
  deps: Deps,
  explicit: string | undefined,
): Promise<SlotStatusReport> {
  const poolDir = await poolDirFor(deps);
  const target = await resolveTargetSlot(deps, explicit);
  await validatePoolSlot(deps, target, poolDir);
  return slotStatus(deps, target);
}

/** `bs lock [PATH]`; returns the locked path. */
export async function lockSlot(
  deps: Deps,
  explicit: string | undefined,
  reason: string | undefined,
): Promise<string> {
  const poolDir = await poolDirFor(deps);
  const target = await resolveTargetSlot(deps, explicit);
  await validatePoolSlot(deps, target, poolDir);

  deps.log.info(
    reason === undefined
      ? `Locking slot ${target}`
      : `Locking slot ${target} with reason: ${reason}`,
  );
  try {
    await deps.git.worktreeLock(target, reason);
  } catch (e) {
    deps.log.warn(`Failed to lock slot ${target}: ${errorMessage(e)}`);
    throw e;
  }
  deps.log.debug(`Successfully locked slot ${target}`);
  return target;
}

/** `bs unlock [PATH]`; returns the unlocked path. */
export async function unlockSlot(
  deps: Deps,
  explicit: string | undefined,
): Promise<string> {
  const poolDir = await poolDirFor(deps);
  const target = await resolveTargetSlot(deps, explicit);
  await validatePoolSlot(deps, target, poolDir);

  deps.log.info(`Unlocking slot ${target}`);
  try {
    await deps.git.worktreeUnlock(target);
  } catch (e) {
    deps.log.warn(`Failed to unlock slot ${target}: ${errorMessage(e)}`);
    throw e;
  }
  deps.log.debug(`Successfully unlocked slot ${target}`);
  return target;
}

// ── prune ──────────────────────────────────────────────────────────────────

/**
 * Prune available slots, always keeping the pool warm: preserve exactly one
 * (the first in pool order, detaching its branch if it has one), delete the
 * other available slots' directories, then run `git worktree prune` once.
 *
 * Returns `undefined` when the pool directory does not exist. Individual
 * deletion/detach failures are captured in the outcome, not thrown.
 */
export async function prunePool(deps: Deps): Promise<PruneOutcome | undefined> {
  const poolDir = await poolDirFor(deps);
  if (!(await deps.fs.exists(poolDir))) return undefined;

  const available = (await listWorktreesStatus(deps, poolDir)).filter(
    (e) => e.status === "available",
  );
  const [preserved, ...toDelete] = available;

  const outcome: PruneOutcome = {
    pruned: [],
    failures: [],
    preserved: undefined,
    preserveFailure: undefined,
  };

  if (preserved !== undefined) {
    const slot: PrunedSlot = { path: preserved.path, branch: preserved.branch };
    if (preserved.branch === undefined) {
      outcome.preserved = slot;
    } else {
      try {
        const headSha = await deps.git.resolveHead();
        await resetSlot(deps, preserved.path, headSha, undefined);
        outcome.preserved = slot;
      } catch (e) {
        outcome.preserveFailure = {
          path: preserved.path,
          error: errorMessage(e),
        };
      }
    }
  }

  for (const entry of toDelete) {
    try {
      await deps.fs.removeDir(entry.path);
      outcome.pruned.push({ path: entry.path, branch: entry.branch });
    } catch (e) {
      outcome.failures.push({
        path: entry.path,
        error: `failed to delete slot directory ${entry.path}: ${errorMessage(e)}`,
      });
    }
  }

  await deps.git.worktreePrune();
  return outcome;
}

// ── info ───────────────────────────────────────────────────────────────────

export interface Info {
  version: string;
  logLevel: string;
  logDirectory: string;
  currentLogFile: string;
  managedRoot: string;
}

/** `bs info` data: resolved paths and metadata, with no filesystem writes. */
export function getInfo(
  deps: Deps,
  logs: LogLocation,
  meta: { version: string; logLevel: string },
): Info {
  const logDir = logs.dir();
  if (logDir === undefined) {
    throw new BonsaiError("Failed to resolve log directory");
  }
  return {
    version: meta.version,
    logLevel: meta.logLevel,
    logDirectory: tilde(deps, logDir),
    currentLogFile: tilde(deps, logs.currentFile(logDir)),
    managedRoot: tilde(deps, managedRoot(deps)),
  };
}

export { poolDirFor };
