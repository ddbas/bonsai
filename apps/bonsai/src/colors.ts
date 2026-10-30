import pc from "picocolors";

/**
 * Colours are always emitted (as before the TypeScript migration), so output
 * is identical whether or not stdout is a terminal.
 */
export const colors = pc.createColors(true);
