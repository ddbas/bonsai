import type {
  GitStatusDetail,
  GitStatusLine,
  WorktreeStatus,
} from "./types.ts";

/**
 * Classify a slot given its lock state, dirty state, and whether its branch
 * carries unmerged work. Priority: `locked` > `in use` > `available`.
 *
 * This is the single place the priority rule is encoded; list, status, prune
 * and get all route through it so the commands can never disagree.
 */
export function classify(
  locked: boolean,
  dirty: boolean,
  hasUnmergedWork: boolean,
): WorktreeStatus {
  if (locked) return "locked";
  if (dirty || hasUnmergedWork) return "in use";
  return "available";
}

/** Lower-case ASCII slug: every non-alphanumeric character becomes `-`. */
export function slugify(name: string): string {
  let out = "";
  for (const ch of name) {
    out += /^[A-Za-z0-9]$/.test(ch) ? ch.toLowerCase() : "-";
  }
  return out;
}

/**
 * Parse a `locked` line from `git worktree list --porcelain`: `locked` or
 * `locked <reason>`. Returns `[isLocked, reason]`.
 */
export function parseLockedLine(line: string): [boolean, string | undefined] {
  if (!line.startsWith("locked")) return [false, undefined];
  const reason = line.slice("locked".length).trim();
  return [true, reason === "" ? undefined : reason];
}

/**
 * Split `git status --porcelain` text into uncommitted (non-`??`) and
 * untracked (`??`) lines.
 */
export function parseStatusPorcelain(text: string): GitStatusDetail {
  const detail: GitStatusDetail = { uncommitted: [], untracked: [] };
  for (const line of text.split("\n")) {
    if (line === "") continue;
    const entry: GitStatusLine =
      line.length >= 3
        ? { code: line.slice(0, 2), path: line.slice(3) }
        : line.length >= 2
          ? { code: line.slice(0, 2), path: "" }
          : { code: line, path: "" };
    (entry.code === "??" ? detail.untracked : detail.uncommitted).push(entry);
  }
  return detail;
}
