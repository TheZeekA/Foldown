function normalizeSegments(path: string): string | null {
  const prefix = path.match(/^[A-Za-z]:/)?.[0] ?? "";
  const parts = path.replace(/\\/g, "/").split("/");
  const output: string[] = [];
  for (const part of parts) {
    if (!part || part === ".") continue;
    if (part.toLowerCase() === prefix.toLowerCase()) continue;
    if (part === "..") {
      if (output.length === 0) return null;
      output.pop();
    } else {
      output.push(part);
    }
  }
  return `${prefix}${prefix ? "/" : ""}${output.join("/")}`;
}

/** A protocol-relative URL (`//host/path`) has no scheme of its own — the
 * browser resolves it against the *current page's* scheme. Foldown's pages
 * are served from an `http:` origin, so a value like `//asset.localhost/...`
 * would resolve to the app's own wildcard-scoped asset protocol, letting a
 * malicious document read arbitrary local files as an "external" image. Treat
 * it as unsafe rather than as an absolute/external reference to skip. */
export function isProtocolRelative(value: string): boolean {
  return /^\/\//.test(value);
}

/** Only a real URI scheme (2+ letters, e.g. `https:`, `mailto:`, `data:`)
 * counts as external — a single letter followed by `:` is a Windows drive
 * letter (`C:\...`), not a scheme, and must still be treated as a local path. */
function hasExternalScheme(value: string): boolean {
  return /^[a-z]{2,}[a-z\d+.-]*:/i.test(value);
}

export function resolveLocalImagePath(src: string, markdownPath: string, workspaceRoot: string): string | null {
  const value = src.trim();
  if (!value || value.startsWith("#") || isProtocolRelative(value) || hasExternalScheme(value)) return null;
  const encodedPath = value.split(/[?#]/, 1)[0] ?? "";
  let withoutQuery = encodedPath;
  try { withoutQuery = decodeURIComponent(encodedPath); } catch { /* retain the original path */ }
  const root = normalizeSegments(workspaceRoot);
  if (!root) return null;
  const rootKey = root.toLowerCase().replace(/\/$/, "");

  // An absolute local path (a Windows drive letter, e.g. "C:\Users\x.png")
  // isn't relative to the document at all — resolve it directly against the
  // workspace root instead of joining it onto the markdown file's directory.
  if (/^[A-Za-z]:[\\/]/.test(withoutQuery)) {
    const absolute = normalizeSegments(withoutQuery);
    return absolute && absolute.toLowerCase().startsWith(`${rootKey}/`) ? absolute : null;
  }

  const markdown = normalizeSegments(markdownPath);
  if (!markdown) return null;
  const markdownKey = markdown.toLowerCase();
  const markdownRelative = markdownKey.startsWith(`${rootKey}/`) ? markdown.slice(root.length + 1) : markdown;
  if (/^[A-Za-z]:\//.test(markdownRelative)) return null;
  const markdownDir = markdownRelative.split("/").slice(0, -1).join("/");
  const relative = normalizeSegments(`${markdownDir}/${withoutQuery}`);
  if (!relative) return null;
  const absolute = normalizeSegments(`${root}/${relative}`);
  if (!absolute || !absolute.toLowerCase().startsWith(`${rootKey}/`)) return null;
  return absolute;
}
