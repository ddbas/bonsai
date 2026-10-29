/**
 * Branch-state check: does a clean slot's branch carry unmerged work?
 *
 * Uses only local git state (refs, config, objects): no network, no fetch.
 * Whenever merged-ness cannot be proven the answer errs toward "unmerged"
 * (the slot is `in use`).
 */
import type { Git } from "./ports.ts";

/** The ref a slot's work is compared against; `undefined` = unresolvable. */
export type DefaultRef = string | undefined;

/** Outcome of the branch-state check for one slot. */
export type BranchState =
  /** HEAD is an ancestor of the default ref: nothing unmerged. */
  | { kind: "no-own-commits" }
  /** Content is on the default ref, branch was pushed, remote branch gone. */
  | { kind: "merged-remote-gone" }
  /** N commits not on default and no upstream configured. */
  | { kind: "unpushed"; n: number }
  /** N commits not on default and the remote-tracking branch still exists. */
  | { kind: "remote-exists"; n: number }
  /** N commits not on default, remote gone, but content not on default. */
  | { kind: "not-merged"; n: number }
  /** Detached HEAD with N commits not on default. */
  | { kind: "detached-with-commits"; n: number }
  /** No default ref could be resolved. */
  | { kind: "default-unknown" };

/** `true` when resetting the slot cannot lose work. */
export function isAvailable(state: BranchState): boolean {
  return state.kind === "no-own-commits" || state.kind === "merged-remote-gone";
}

function commits(n: number): string {
  return n === 1 ? "1 commit" : `${n} commits`;
}

/** Human-readable reason, relative to `defaultRef`. */
export function reason(state: BranchState, defaultRef: DefaultRef): string {
  const d = defaultRef ?? "the default branch";
  switch (state.kind) {
    case "no-own-commits":
      return "no commits of its own";
    case "merged-remote-gone":
      return `merged into ${d}, remote branch is gone`;
    case "unpushed":
      return `${commits(state.n)} ahead of ${d}, no upstream configured (push with \`-u\` to enable merged-branch detection)`;
    case "remote-exists":
      return `${commits(state.n)} ahead of ${d}, remote branch still exists`;
    case "not-merged":
      return `${commits(state.n)} ahead of ${d}, not merged`;
    case "detached-with-commits":
      return `detached HEAD with ${commits(state.n)} not on ${d}`;
    case "default-unknown":
      return "default branch could not be determined";
  }
}

/**
 * Resolve the default ref once: `origin/HEAD`, else `origin/main`,
 * `origin/master`; local `main`/`master` only when there is no `origin`.
 */
export async function resolveDefaultRef(
  git: Git,
  dir: string,
): Promise<DefaultRef> {
  const head = await git.symbolicRefShort(dir, "refs/remotes/origin/HEAD");
  if (head !== null && head !== "" && (await git.refExists(dir, head))) {
    return head;
  }
  for (const c of ["origin/main", "origin/master"]) {
    if (await git.refExists(dir, `refs/remotes/${c}`)) return c;
  }
  if ((await git.remoteUrl(dir, "origin")) === null) {
    for (const c of ["main", "master"]) {
      if (await git.refExists(dir, `refs/heads/${c}`)) return c;
    }
  }
  return undefined;
}

/**
 * A default ref resolved at most once, on first use, and shareable across
 * concurrent per-slot checks (locked/dirty slots never pay for resolution).
 */
export class LazyDefaultRef {
  private pending: Promise<DefaultRef> | undefined;

  constructor(
    private readonly git: Git,
    private readonly dir: string,
  ) {}

  get(): Promise<DefaultRef> {
    this.pending ??= resolveDefaultRef(this.git, this.dir);
    return this.pending;
  }
}

/** Is the branch's content already on `d`? (cherry for rebase, merge-tree for squash) */
async function contentOnDefault(
  git: Git,
  slot: string,
  d: string,
): Promise<boolean> {
  const cherry = await git.cherry(slot, d);
  if (cherry !== null && !cherry.split("\n").some((l) => l.startsWith("+"))) {
    return true;
  }
  const resultTree = await git.mergeTree(slot, d, "HEAD");
  if (resultTree === null) return false;
  return (await git.treeOf(slot, d)) === resultTree;
}

/**
 * Evaluate the branch state of `slot` (checked-out `branch`, `undefined` if
 * detached), cheapest check first with early return.
 */
export async function branchState(
  git: Git,
  slot: string,
  branch: string | undefined,
  defaultRef: DefaultRef,
): Promise<BranchState> {
  if (defaultRef === undefined) return { kind: "default-unknown" };
  const d = defaultRef;

  if (await git.isAncestor(slot, "HEAD", d)) return { kind: "no-own-commits" };

  const n = (await git.countCommits(slot, `${d}..HEAD`)) ?? 0;
  if (branch === undefined) return { kind: "detached-with-commits", n };

  const remote = await git.configValue(slot, `branch.${branch}.remote`);
  const merge = await git.configValue(slot, `branch.${branch}.merge`);
  if (remote === null || merge === null) return { kind: "unpushed", n };
  if (remote === "." || remote === "" || merge === "") {
    return { kind: "unpushed", n };
  }

  const mergeBranch = merge.startsWith("refs/heads/")
    ? merge.slice("refs/heads/".length)
    : merge;
  if (await git.refExists(slot, `refs/remotes/${remote}/${mergeBranch}`)) {
    return { kind: "remote-exists", n };
  }

  return (await contentOnDefault(git, slot, d))
    ? { kind: "merged-remote-gone" }
    : { kind: "not-merged", n };
}
