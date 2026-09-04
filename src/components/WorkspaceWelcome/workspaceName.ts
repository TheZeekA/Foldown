export function workspaceNameError(value: string): string | null {
  if (!value || value.trim() !== value || value === "." || value === ".." || /[<>:"/\\|?*\u0000-\u001f]/.test(value) || /[. ]$/.test(value)) {
    return "Enter a valid folder name without path separators or Windows reserved characters.";
  }
  return null;
}
