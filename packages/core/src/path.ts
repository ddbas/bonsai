/**
 * Minimal, pure POSIX path helpers. Core must not import `node:path`, and
 * bonsai only supports POSIX paths.
 */

export function isAbsolute(p: string): boolean {
  return p.startsWith("/");
}

/** Resolve `p` against `base` (when relative) and normalise `.`/`..`/`//`. */
export function resolve(base: string, p: string): string {
  const full = isAbsolute(p) ? p : `${base}/${p}`;
  const out: string[] = [];
  for (const part of full.split("/")) {
    if (part === "" || part === ".") continue;
    if (part === "..") out.pop();
    else out.push(part);
  }
  return `/${out.join("/")}`;
}

/** Join path segments with `/` (no normalisation, like Rust's `Path::join`). */
export function join(base: string, ...parts: string[]): string {
  let out = base;
  for (const part of parts) {
    if (isAbsolute(part)) out = part;
    else out = out.endsWith("/") ? `${out}${part}` : `${out}/${part}`;
  }
  return out;
}

/** Parent directory, or `undefined` for the root / an empty path. */
export function dirname(p: string): string | undefined {
  const trimmed = p.length > 1 ? p.replace(/\/+$/, "") : p;
  if (trimmed === "/" || trimmed === "") return undefined;
  const idx = trimmed.lastIndexOf("/");
  if (idx < 0) return "";
  return idx === 0 ? "/" : trimmed.slice(0, idx);
}

/** Final path component, or `undefined` when there is none (e.g. `/`). */
export function basename(p: string): string | undefined {
  const trimmed = p.replace(/\/+$/, "");
  if (trimmed === "") return undefined;
  const name = trimmed.slice(trimmed.lastIndexOf("/") + 1);
  return name === "" || name === ".." ? undefined : name;
}

/** Component-wise prefix test (`/a/bc` does not start with `/a/b`). */
export function startsWith(p: string, prefix: string): boolean {
  if (p === prefix) return true;
  const base = prefix.endsWith("/") ? prefix : `${prefix}/`;
  return p.startsWith(base);
}

/** Replace a leading `home` with `~` (exactly `home` becomes `~/`). */
export function tildePath(p: string, home: string | undefined): string {
  if (home !== undefined && home !== "") {
    if (p === home) return "~/";
    if (startsWith(p, home)) {
      const rest = p.slice(home.endsWith("/") ? home.length : home.length + 1);
      return `~/${rest}`;
    }
  }
  return p;
}
