/** A registered worktree as reported by `git worktree list --porcelain`. */
export interface WorktreeEntry {
  path: string;
  locked: boolean;
  /** Reason passed to `git worktree lock --reason`, when locked with one. */
  lockReason: string | undefined;
  /** Short checked-out branch name, or `undefined` for detached HEAD. */
  branch: string | undefined;
}

/** A single classified line from `git status --porcelain`. */
export interface GitStatusLine {
  /** Two-character XY code (e.g. ` M`, `??`, `A `). */
  code: string;
  /** Path as reported by git (renames keep their ` -> ` text verbatim). */
  path: string;
}

/** `git status --porcelain` output split into uncommitted and untracked. */
export interface GitStatusDetail {
  uncommitted: GitStatusLine[];
  untracked: GitStatusLine[];
}

export type WorktreeStatus = "available" | "in use" | "locked";

/** Detailed status report for a single managed pool slot. */
export interface SlotStatusReport {
  path: string;
  branch: string | undefined;
  status: WorktreeStatus;
  lockReason: string | undefined;
  /** Human-readable branch-state reason (`undefined` for locked slots). */
  branchState: string | undefined;
  gitStatus: GitStatusDetail;
}

/** One row of `bs list`. */
export interface WorktreeListEntry {
  path: string;
  status: WorktreeStatus;
  branch: string | undefined;
}

/** How a branch is created/checked out inside the provisioned slot. */
export type BranchMode =
  /** `-b`: create a new branch; fails if it exists. */
  | { kind: "new"; name: string }
  /** `-B`: create or reset a branch. */
  | { kind: "reset"; name: string }
  /** Positional: check out an existing branch. */
  | { kind: "existing"; name: string };

/** A pool slot that `bs prune` deleted (or preserved). */
export interface PrunedSlot {
  path: string;
  branch: string | undefined;
}

export interface PruneOutcome {
  pruned: PrunedSlot[];
  failures: { path: string; error: string }[];
  /** The available slot kept to keep the pool warm, if any existed. */
  preserved: PrunedSlot | undefined;
  /** Set when detaching the preserved slot's branch failed. */
  preserveFailure: { path: string; error: string } | undefined;
}
