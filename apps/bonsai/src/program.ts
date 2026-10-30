/** Commander program: argument parsing, output formatting, no business logic. */
import {
  BonsaiError,
  type BranchMode,
  currentWorktree,
  DETACHED_LABEL,
  ensureSession,
  errorMessage,
  getInfo,
  getWorktree,
  listPool,
  lockSlot,
  prunePool,
  repoSlug,
  resolveSessionName,
  statusForPath,
  tilde,
  unlockSlot,
} from "@bonsai/core";
import { isLogLevel, LOG_LEVELS, type LogLevel } from "@bonsai/logging";
import { Command, Option } from "commander";
import {
  formatCurrentPath,
  formatListRow,
  formatStatusReport,
} from "./format.ts";
import type { Io } from "./io.ts";
import type { Runtime, RuntimeFactory } from "./runtime.ts";

export interface ProgramOptions {
  version: string;
  runtime: RuntimeFactory;
  io: Io;
  /** Make commander throw instead of calling `process.exit` (tests). */
  exitOverride?: boolean;
}

export interface BsProgram {
  program: Command;
  /** Release runtime resources (flush the log file). Safe to call twice. */
  close(): void;
  /** Exit code requested by the last action (0 unless it failed). */
  exitCode(): number;
}

interface GetOptions {
  b?: string;
  B?: string;
  tmuxSession?: string | true;
  attach: boolean;
}

export function buildProgram(options: ProgramOptions): BsProgram {
  const { io } = options;
  let runtime: Runtime | undefined;
  let exitCode = 0;

  const program = new Command();
  program
    .name("bs")
    .description(
      "🌳 bonsai – provision clean git worktrees for fast context-switching.",
    )
    .helpCommand(false)
    .addOption(
      new Option(
        "--log-level <LEVEL>",
        "Log level for the log file: trace, debug, info, warn, or error (default: info). Does not affect stdout/stderr output.",
      )
        .choices(LOG_LEVELS)
        .default("info"),
    )
    .configureOutput({
      writeOut: (s) => io.out(s),
      writeErr: (s) => io.err(s),
    });
  if (options.exitOverride) program.exitOverride();

  const logLevel = (): LogLevel => {
    const value = program.opts<{ logLevel: string }>().logLevel;
    return isLogLevel(value) ? value : "info";
  };

  /** The runtime (logging, git, …) is created lazily, once, after parsing. */
  const rt = (): Runtime => {
    runtime ??= options.runtime({ logLevel: logLevel() });
    return runtime;
  };

  const println = (line = ""): void => io.out(`${line}\n`);

  /** Wrap an action: report expected failures as `error: …` and exit 1. */
  const action =
    <A extends unknown[]>(fn: (...args: A) => Promise<void>) =>
    async (...args: A): Promise<void> => {
      try {
        await fn(...args);
      } catch (e) {
        if (!(e instanceof BonsaiError) && !(e instanceof Error)) throw e;
        io.err(`error: ${errorMessage(e)}\n`);
        exitCode = 1;
      }
    };

  program.hook("preAction", () => {
    rt().deps.log.debug(`CLI invocation: bs ${JSON.stringify(process.argv)}`);
  });

  // Default command (no subcommand): always detached HEAD.
  program.action(
    action(async () => {
      const path = await getWorktree(rt().deps, undefined);
      println(`🌳 ${path}`);
    }),
  );

  // ── get ────────────────────────────────────────────────────────────────
  program
    .command("get")
    .description(
      `Provision or reuse a managed git worktree from the pool, printing its path.

Resets the slot to the current HEAD. Use \`-b <branch>\` to create a new branch, \`-B <branch>\` to create-or-reset a branch, or pass a positional \`<branch>\` to check out an existing branch; these three are mutually exclusive, and with none the slot is left in detached HEAD state.

This is the implicit default command: running \`bs\` alone is equivalent to \`bs get\` in detached HEAD state.`,
    )
    .argument(
      "[branch]",
      "Check out an existing branch inside the provisioned slot. Fails if the branch does not exist. Mutually exclusive with `-b`/`-B`.",
    )
    .addOption(
      new Option(
        "-b <branch>",
        "Create a new branch at HEAD in the provisioned slot. Fails if the branch already exists. Mutually exclusive with `-B`.",
      ).conflicts("B"),
    )
    .addOption(
      new Option(
        "-B <branch>",
        "Create or reset a branch at HEAD in the provisioned slot, overwriting an existing branch without error. Mutually exclusive with `-b`.",
      ).conflicts("b"),
    )
    .option(
      "--tmux-session [name]",
      "Create (or reuse) a tmux session rooted at the provisioned slot. With no value, a session name is generated automatically; pass `--tmux-session=NAME` to use a specific name. Requires `tmux` on `PATH`.",
    )
    .option(
      "--no-attach",
      "Create the tmux session in the background without attaching the invoking terminal to it. Requires `--tmux-session`.",
    )
    .action(
      action(
        async (
          positional: string | undefined,
          opts: GetOptions,
          cmd: Command,
        ) => {
          if (
            positional !== undefined &&
            (opts.b !== undefined || opts.B !== undefined)
          ) {
            const flag = opts.b !== undefined ? "-b <branch>" : "-B <branch>";
            cmd.error(
              `error: argument '[branch]' cannot be used with option '${flag}'`,
            );
          }
          if (
            cmd.getOptionValueSource("attach") === "cli" &&
            opts.tmuxSession === undefined
          ) {
            cmd.error(
              "error: option '--no-attach' requires option '--tmux-session [name]'",
            );
          }

          // Bare `--tmux-session` → empty string sentinel (generated name).
          const tmuxValue = opts.tmuxSession === true ? "" : opts.tmuxSession;

          // Check up front (before provisioning) so a missing `tmux` fails
          // fast without leaving a half-completed operation behind.
          if (tmuxValue !== undefined) await rt().tmux.checkAvailable();

          let mode: BranchMode | undefined;
          if (positional !== undefined) {
            mode = { kind: "existing", name: positional };
          } else if (opts.b !== undefined) {
            mode = { kind: "new", name: opts.b };
          } else if (opts.B !== undefined) {
            mode = { kind: "reset", name: opts.B };
          }

          const { deps, tmux } = rt();
          const path = await getWorktree(deps, mode);
          println(
            mode === undefined ? `🌳 ${path}` : `🌳 ${path}  (${mode.name})`,
          );

          if (tmuxValue !== undefined) {
            const repoName = await repoSlug(deps);
            const sessionName = resolveSessionName(
              tmuxValue,
              repoName,
              mode?.name ?? DETACHED_LABEL,
            );
            await ensureSession(tmux, deps.log, sessionName, path);
            println(`\u{1f5a5}\u{fe0f}  tmux session: ${sessionName}`);
            if (opts.attach) await tmux.attach(sessionName);
          }
        },
      ),
    );

  // ── list ───────────────────────────────────────────────────────────────
  program
    .command("list")
    .alias("ls")
    .description(
      `List all managed worktrees in the pool.

Displays one line per slot: a coloured status badge (\`available\`/\`in use\`/\`locked\`), followed by the tilde-abbreviated path, optionally followed by the checked-out branch name in parentheses. The current slot is marked with a \`▶\` prefix (no other worktrees are so marked). Per-slot checks short-circuit as soon as the badge's classification is known: a locked slot never triggers \`git status\` or branch-state checks; a dirty unlocked slot never triggers branch-state checks. No per-file detail (counts or itemized lists) is computed or displayed here — use \`bs status [PATH]\` for that.`,
    )
    .action(
      action(async () => {
        const { deps } = rt();
        const pool = await listPool(deps);
        if (pool === undefined) {
          println(
            "No worktrees managed for this repository (pool does not exist yet).",
          );
          println("Run `bs get` to create the first slot.");
          return;
        }
        if (pool.entries.length === 0) {
          println("No worktrees managed for this repository.");
          println("Run `bs get` to create the first slot.");
          return;
        }
        for (const entry of pool.entries) {
          println(
            formatListRow(
              entry.status,
              tilde(deps, entry.path),
              entry.branch,
              pool.current === entry.path,
            ),
          );
        }
      }),
    );

  // ── status ─────────────────────────────────────────────────────────────
  program
    .command("status")
    .description(
      `Show detailed status for a single managed worktree slot.

Reports the slot's overall classification (\`available\` / \`in use\` / \`locked\`, using the same priority rules \`bs list\`'s status badge applies), plus itemized detail: the lock reason (if locked), the individual \`git status --porcelain\` lines (uncommitted and untracked), and a branch-state line explaining whether the slot's branch carries unmerged work.

\`PATH\` is optional; when omitted, \`bs status\` resolves the slot containing the current working directory (same resolution as \`bs current\`) and errors out if the CWD is not inside a managed slot. When given, \`PATH\` must resolve to a bonsai-managed pool slot for the current repository (validated the same way as \`bs lock\`/\`bs unlock\`).`,
    )
    .argument(
      "[path]",
      "Absolute path to the pool slot to inspect. Defaults to the current slot when omitted.",
    )
    .action(
      action(async (path: string | undefined) => {
        const { deps } = rt();
        const report = await statusForPath(deps, path);
        for (const line of formatStatusReport(
          report,
          tilde(deps, report.path),
        )) {
          println(line);
        }
      }),
    );

  // ── current ────────────────────────────────────────────────────────────
  program
    .command("current")
    .description(
      "Show the managed worktree slot that contains the current directory.\n\nExits with status 0 when inside a managed slot, 1 otherwise.",
    )
    .action(
      action(async () => {
        const { deps } = rt();
        const current = await currentWorktree(deps);
        if (current === undefined) {
          println("Not inside a managed bonsai worktree.");
          println("Run `bs get` to provision a slot, then `cd` into it.");
          throw new BonsaiError("not inside a managed bonsai worktree");
        }
        println(
          `🌳 ${formatCurrentPath(tilde(deps, current.path), current.branch)}`,
        );
      }),
    );

  // ── help ───────────────────────────────────────────────────────────────
  program
    .command("help")
    .description("Show usage information.")
    .action(() => {
      program.outputHelp();
    });

  // ── lock / unlock ──────────────────────────────────────────────────────
  program
    .command("lock")
    .description(
      "Lock a bonsai pool slot, preventing `bs get` from reusing it.\n\nDefaults to the current slot when no path argument is supplied.",
    )
    .option(
      "--reason <MESSAGE>",
      "Reason stored with the lock, forwarded to git verbatim.",
    )
    .argument(
      "[path]",
      "Path to the pool slot to lock. Defaults to the current slot.",
    )
    .action(
      action(async (path: string | undefined, opts: { reason?: string }) => {
        const { deps } = rt();
        const target = await lockSlot(deps, path, opts.reason);
        println(`\u{1f512} locked ${tilde(deps, target)}`);
      }),
    );

  program
    .command("unlock")
    .description(
      "Unlock a bonsai pool slot, making it available for reuse by `bs get`.\n\nDefaults to the current slot when no path argument is supplied.",
    )
    .argument(
      "[path]",
      "Path to the pool slot to unlock. Defaults to the current slot.",
    )
    .action(
      action(async (path: string | undefined) => {
        const { deps } = rt();
        const target = await unlockSlot(deps, path);
        println(`\u{1f513} unlocked ${tilde(deps, target)}`);
      }),
    );

  // ── info ───────────────────────────────────────────────────────────────
  program
    .command("info")
    .description(
      "Print bonsai's runtime paths and metadata.\n\nPrints resolved paths (log directory, current log file, managed root) and metadata (version, effective log level) as `key: value` lines. Performs no filesystem writes.",
    )
    .action(
      action(async () => {
        const { deps, logs } = rt();
        const info = getInfo(deps, logs, {
          version: options.version,
          logLevel: logLevel(),
        });
        println(`version: ${info.version}`);
        println(`log level: ${info.logLevel}`);
        println(`log directory: ${info.logDirectory}`);
        println(`current log file: ${info.currentLogFile}`);
        println(`managed root: ${info.managedRoot}`);
      }),
    );

  // ── prune ──────────────────────────────────────────────────────────────
  program
    .command("prune")
    .description(
      "Reclaim disk space by deleting unused (`available`) bonsai pool slots for the current repository, then running `git worktree prune`.\n\nClassifies every managed pool slot using the same `available` / `in use` / `locked` rules as `bs list`/`bs status`. Among slots classified `available`, exactly one is always preserved (kept on disk and registered) to keep the pool warm — detaching its branch first if it had one checked out — and the rest have their on-disk directories deleted; `locked` and `in use` slots are never touched.\n\nOptional PATH arguments narrow which slots are pruned (invalid, locked, or — without --force — in-use targets abort the run before anything changes). With --force, `in use` slots are pruned too (uncommitted and untracked work is discarded; branches are never deleted); if one must be preserved it is reset, cleaned and detached so it is `available`. `locked` slots are never pruned or modified, even with --force; unlock them first. After deleting, `git worktree prune` is run exactly once so git deregisters the removed slots — `bs prune` never edits git's worktree bookkeeping directly. Prints the tilde-abbreviated path (and branch, if any) of each pruned slot. Exits 0 unless a slot's directory failed to delete or the preserved slot failed to detach.",
    )
    .argument("[paths...]", "only prune the slots at these paths")
    .option(
      "--force",
      "also prune `in use` slots (discarding uncommitted work); `locked` slots are never touched",
    )
    .action(
      action(async (paths: string[], opts: { force?: boolean }) => {
        const { deps } = rt();
        const outcome = await prunePool(deps, {
          paths,
          force: opts.force === true,
        });
        if (outcome === undefined) {
          println(
            "No worktrees managed for this repository (pool does not exist yet).",
          );
          println("Nothing to prune.");
          return;
        }

        if (
          outcome.pruned.length === 0 &&
          outcome.failures.length === 0 &&
          outcome.preserveFailure === undefined
        ) {
          println("Nothing to prune (no available slots).");
          return;
        }

        for (const slot of outcome.pruned) {
          println(
            `\u{1f5d1}\u{fe0f}  pruned ${formatCurrentPath(tilde(deps, slot.path), slot.branch)}`,
          );
        }

        let failureCount = outcome.failures.length;
        const total =
          outcome.pruned.length +
          outcome.failures.length +
          (outcome.preserved === undefined ? 0 : 1) +
          (outcome.preserveFailure === undefined ? 0 : 1);

        for (const f of outcome.failures) {
          io.err(`error: failed to prune ${tilde(deps, f.path)}: ${f.error}\n`);
        }
        if (outcome.preserveFailure !== undefined) {
          failureCount++;
          io.err(
            `error: failed to detach or clean preserved slot ${tilde(deps, outcome.preserveFailure.path)}: ${outcome.preserveFailure.error}\n`,
          );
        }

        if (failureCount > 0) {
          throw new BonsaiError(
            `failed to prune ${failureCount} of ${total} available slot(s)`,
          );
        }
      }),
    );

  // The command list shows only each command's first paragraph.
  for (const cmd of program.commands) {
    cmd.summary((cmd.description().split("\n")[0] ?? "").trim());
  }

  return {
    program,
    close: () => {
      runtime?.close();
      runtime = undefined;
    },
    exitCode: () => exitCode,
  };
}
