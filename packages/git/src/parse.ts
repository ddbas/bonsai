import { parseLockedLine, type WorktreeEntry } from "@bonsai/core";

/**
 * Parse `git worktree list --porcelain` into entries (paths exactly as git
 * reports them; no canonicalisation).
 */
export function parseWorktreeList(text: string): WorktreeEntry[] {
  const entries: WorktreeEntry[] = [];
  let current: WorktreeEntry | undefined;

  for (const line of text.split("\n")) {
    if (line.startsWith("worktree ")) {
      current = {
        path: line.slice("worktree ".length).trim(),
        locked: false,
        lockReason: undefined,
        branch: undefined,
      };
      entries.push(current);
    } else if (current !== undefined && line.startsWith("locked")) {
      [current.locked, current.lockReason] = parseLockedLine(line);
    } else if (current !== undefined && line.startsWith("branch ")) {
      const ref = line.slice("branch ".length).trim();
      current.branch = ref.startsWith("refs/heads/")
        ? ref.slice("refs/heads/".length)
        : ref;
    }
    // `detached` line → branch stays undefined
  }
  return entries;
}
