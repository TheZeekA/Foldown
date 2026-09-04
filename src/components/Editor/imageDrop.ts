const SUPPORTED_IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg"]);

export function isSupportedImagePath(path: string): boolean {
  const extension = path.split(/[\\/.]/).pop()?.toLowerCase() ?? "";
  return SUPPORTED_IMAGE_EXTENSIONS.has(extension);
}

export function buildImageMarkdown(assetPath: string): string {
  const filename = assetPath.split(/[\\/]/).pop() ?? assetPath;
  const stem = filename.replace(/\.[^.]+$/, "").replace(/[-_]+/g, " ").trim() || "image";
  const normalizedPath = assetPath.replace(/\\/g, "/");
  const destination = /\s/.test(normalizedPath) ? `<${normalizedPath}>` : normalizedPath;
  return `![${stem}](${destination})`;
}

export function buildImageMarkdownForDocument(assetPath: string, markdownPath: string, workspaceRoot: string): string {
  const asset = assetPath.replace(/\\/g, "/");
  const root = workspaceRoot.replace(/\\/g, "/").replace(/\/$/, "");
  const document = markdownPath.replace(/\\/g, "/");
  const assetRelative = asset.toLowerCase().startsWith(`${root.toLowerCase()}/`) ? asset.slice(root.length + 1) : asset;
  const documentRelative = document.toLowerCase().startsWith(`${root.toLowerCase()}/`) ? document.slice(root.length + 1) : document;
  const fromParts = documentRelative.split("/").slice(0, -1).filter(Boolean);
  const targetParts = assetRelative.split("/").filter(Boolean);
  while (fromParts.length && targetParts.length && fromParts[0].toLowerCase() === targetParts[0].toLowerCase()) {
    fromParts.shift();
    targetParts.shift();
  }
  const relative = `${[...fromParts.map(() => ".."), ...targetParts].join("/")}` || assetRelative;
  return buildImageMarkdown(relative);
}
