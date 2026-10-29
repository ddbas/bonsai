/** Strip ANSI CSI escape sequences (colour codes) from a string. */
export function stripAnsi(s: string): string {
  // biome-ignore lint/suspicious/noControlCharactersInRegex: matching ESC
  return s.replace(/\u001b\[[0-9;]*[A-Za-z]/g, "");
}
