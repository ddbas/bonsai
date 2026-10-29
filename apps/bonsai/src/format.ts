import type { SlotStatusReport, WorktreeStatus } from "@bonsai/core";
import { colors } from "./colors.ts";

/** Fixed badge column width: the longest plain badge label ("available"). */
export const BADGE_WIDTH = 9;

const BADGE_COLOR: Record<WorktreeStatus, (s: string) => string> = {
  locked: colors.yellow,
  "in use": colors.red,
  available: colors.green,
};

/** `<tilde>  (<bold branch>)`, or just `<tilde>` when detached. */
export function formatCurrentPath(
  tilde: string,
  branch: string | undefined,
): string {
  return branch === undefined ? tilde : `${tilde}  (${colors.bold(branch)})`;
}

/** One `bs list` row: marker, colour-coded padded badge, path (+ branch). */
export function formatListRow(
  status: WorktreeStatus,
  tilde: string,
  branch: string | undefined,
  isCurrent: boolean,
): string {
  const prefix = isCurrent ? "▶ " : "  ";
  const pathDisplay =
    branch === undefined ? tilde : `${tilde} (${colors.bold(branch)})`;
  // Pad the *plain* label, then colour, so ANSI codes never affect alignment.
  const badge = BADGE_COLOR[status](status.padEnd(BADGE_WIDTH));
  return `${prefix}${badge}  ${pathDisplay}`;
}

/**
 * Render a status report in the plain-text, itemized format of `bs status`:
 * header, classification (+ lock reason), branch state, then itemized
 * uncommitted/untracked sections (omitted when empty).
 */
export function formatStatusReport(
  report: SlotStatusReport,
  tilde: string,
): string[] {
  const lines: string[] = [`🌳 ${formatCurrentPath(tilde, report.branch)}`];

  switch (report.status) {
    case "locked":
      lines.push(`status: ${colors.yellow("locked")}`);
      lines.push(`lock reason: ${report.lockReason ?? "none"}`);
      break;
    case "in use":
      lines.push(`status: ${colors.red("in use")}`);
      break;
    case "available":
      lines.push(`status: ${colors.green("available")}`);
      break;
  }

  if (report.branchState !== undefined) {
    lines.push(`branch state: ${report.branchState}`);
  }

  const { uncommitted, untracked } = report.gitStatus;
  if (uncommitted.length > 0) {
    lines.push("", `uncommitted changes (${uncommitted.length}):`);
    for (const l of uncommitted) lines.push(`  ${l.code}  ${l.path}`);
  }
  if (untracked.length > 0) {
    lines.push("", `untracked files (${untracked.length}):`);
    for (const l of untracked) lines.push(`  ${l.code}  ${l.path}`);
  }
  return lines;
}
