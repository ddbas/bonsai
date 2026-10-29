import type { Logger, Tmux } from "./ports.ts";

/** The literal branch-display label used when no branch was requested. */
export const DETACHED_LABEL = "detached";

/** Default tmux session name: `🌳 <repo-name> (<branch-display>)`. */
export function defaultSessionName(
  repoName: string,
  branchDisplay: string,
): string {
  return `\u{1f333} ${repoName} (${branchDisplay})`;
}

/**
 * Resolve the session name for `--tmux-session`. An empty value (the bare
 * flag sentinel) derives the default name; anything else is used verbatim.
 */
export function resolveSessionName(
  value: string,
  repoName: string,
  branchDisplay: string,
): string {
  return value === "" ? defaultSessionName(repoName, branchDisplay) : value;
}

/** Create a detached session rooted at `cwd`, unless it already exists. */
export async function ensureSession(
  tmux: Tmux,
  log: Logger,
  name: string,
  cwd: string,
): Promise<void> {
  if (await tmux.hasSession(name)) {
    log.debug(`Reusing existing tmux session ${name}`);
    return;
  }
  log.info(`Creating tmux session ${name} at ${cwd}`);
  await tmux.newSession(name, cwd);
}
