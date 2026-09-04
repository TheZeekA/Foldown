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

export function resolveLocalImagePath(src: string, markdownPath: string, workspaceRoot: string): string | null {
  const value = src.trim();
  if (!value || /^(?:[a-z][a-z\d+.-]*:|#|\/\/)/i.test(value)) return null;
  const encodedPath = value.split(/[?#]/, 1)[0] ?? "";
  let withoutQuery = encodedPath;
  try { withoutQuery = decodeURIComponent(encodedPath); } catch { /* retain the original path */ }
  const root = normalizeSegments(workspaceRoot);
  const markdown = normalizeSegments(markdownPath);
  if (!root || !markdown) return null;
  const rootKey = root.toLowerCase().replace(/\/$/, "");
  const markdownKey = markdown.toLowerCase();
  const markdownRelative = markdownKey.startsWith(`${rootKey}/`) ? markdown.slice(root.length + 1) : markdown;
  if (/^[A-Za-z]:\//.test(markdownRelative)) return null;
  const markdownDir = markdownRelative.split("/").slice(0, -1).join("/");
  const relative = normalizeSegments(`${markdownDir}/${withoutQuery}`);
  if (!relative || !root) return null;
  const absolute = normalizeSegments(`${root}/${relative}`);
  if (!absolute || !absolute.toLowerCase().startsWith(`${rootKey}/`)) return null;
  return absolute;
}

export function replaceLocalImageSources(html: string, markdownPath: string, workspaceRoot: string, toAssetUrl: (path: string) => string): string {
  return html.replace(/(<img\b[^>]*\bsrc=")([^"]+)(")/gi, (match, prefix: string, src: string, suffix: string) => {
    const path = resolveLocalImagePath(src, markdownPath, workspaceRoot);
    return path ? `${prefix}${toAssetUrl(path)}${suffix}` : match;
  });
}
