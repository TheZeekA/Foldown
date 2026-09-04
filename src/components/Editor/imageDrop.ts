const SUPPORTED_IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg"]);

export function isSupportedImagePath(path: string): boolean {
  const extension = path.split(/[\\/.]/).pop()?.toLowerCase() ?? "";
  return SUPPORTED_IMAGE_EXTENSIONS.has(extension);
}

export function buildImageMarkdown(assetPath: string): string {
  const filename = assetPath.split(/[\\/]/).pop() ?? assetPath;
  const stem = filename.replace(/\.[^.]+$/, "").replace(/[-_]+/g, " ").trim() || "image";
  return `![${stem}](${assetPath.replace(/\\/g, "/")})`;
}
