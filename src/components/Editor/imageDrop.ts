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
