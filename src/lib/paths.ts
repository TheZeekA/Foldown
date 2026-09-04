/** Minimal path helpers for building/reading paths returned by Rust (which may use \ or /). */

export function joinPath(parent: string, name: string): string {
  return `${parent.replace(/[\\/]+$/, "")}/${name}`;
}

export function dirName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const idx = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return idx === -1 ? trimmed : trimmed.slice(0, idx);
}

export function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? "";
}

export function isSameOrDescendant(candidateParent: string, path: string): boolean {
  // Normalize separators too, not just case/trailing slashes — the two sides
  // routinely come from different sources (joinPath always uses "/", paths
  // read back from the backend are typically "\"-style on Windows), and a
  // mismatch here used to produce false negatives that bypassed callers'
  // safety checks entirely.
  const normalize = (p: string) => p.replace(/[\\/]+$/, "").replace(/\\/g, "/").toLowerCase();
  const a = normalize(candidateParent);
  const b = normalize(path);
  return a === b || a.startsWith(`${b}/`);
}
