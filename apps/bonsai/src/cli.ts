import { processIo } from "./io.ts";
import { buildProgram } from "./program.ts";
import { createRuntime } from "./runtime.ts";

declare const __BONSAI_VERSION__: string | undefined;

const version =
  typeof __BONSAI_VERSION__ === "undefined" ? "0.0.0-dev" : __BONSAI_VERSION__;

async function main(): Promise<void> {
  const bs = buildProgram({ version, runtime: createRuntime, io: processIo });
  try {
    await bs.program.parseAsync(process.argv);
  } finally {
    bs.close();
  }
  process.exitCode = bs.exitCode();
}

main().catch((e: unknown) => {
  process.stderr.write(
    `error: ${e instanceof Error ? e.message : String(e)}\n`,
  );
  process.exitCode = 1;
});
