import type { ThemeMode } from "./types";

/** Matches the default `--font-mono` stack in theme.css — used as the
 * settings panel's "Default" option and the store's initial value. */
export const DEFAULT_EDITOR_FONT_FAMILY =
  '"Cascadia Code", "Consolas", "SFMono-Regular", Menlo, monospace';
/** Matches the editor's current 0.95rem baseline (assuming a 16px root). */
export const DEFAULT_EDITOR_FONT_SIZE = 15;

/** "system" removes the override entirely so the `prefers-color-scheme`
 * media query in theme.css takes over reactively, with no JS involved. */
export function applyTheme(mode: ThemeMode) {
  const root = document.documentElement;
  if (mode === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", mode);
  }
}

export function applyEditorFont(family: string, size: number) {
  const root = document.documentElement;
  root.style.setProperty("--font-mono", family);
  root.style.setProperty("--editor-font-size", `${size}px`);
}
