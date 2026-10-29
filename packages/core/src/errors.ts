/** An expected, user-facing failure. The CLI prints its message and exits 1. */
export class BonsaiError extends Error {
  override readonly name = "BonsaiError";
}

/** Message of any thrown value. */
export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
