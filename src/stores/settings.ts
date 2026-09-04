import { create } from "zustand";
import { getEditorFont, getTheme, setEditorFont, setTheme } from "../lib/tauriApi";
import {
  applyEditorFont,
  applyTheme,
  DEFAULT_EDITOR_FONT_FAMILY,
  DEFAULT_EDITOR_FONT_SIZE,
} from "../lib/theme";
import type { ThemeMode } from "../lib/types";

interface SettingsState {
  theme: ThemeMode;
  editorFontFamily: string;
  editorFontSize: number;

  init: () => Promise<void>;
  setTheme: (theme: ThemeMode) => Promise<void>;
  setEditorFont: (family: string, size: number) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  theme: "system",
  editorFontFamily: DEFAULT_EDITOR_FONT_FAMILY,
  editorFontSize: DEFAULT_EDITOR_FONT_SIZE,

  init: async () => {
    try {
      const [theme, font] = await Promise.all([getTheme(), getEditorFont()]);
      const resolvedTheme = theme ?? "system";
      const resolvedFamily = font?.family ?? DEFAULT_EDITOR_FONT_FAMILY;
      const resolvedSize = font?.size ?? DEFAULT_EDITOR_FONT_SIZE;
      set({ theme: resolvedTheme, editorFontFamily: resolvedFamily, editorFontSize: resolvedSize });
      applyTheme(resolvedTheme);
      applyEditorFont(resolvedFamily, resolvedSize);
    } catch {
      // fall back to the in-memory defaults already applied via CSS
    }
  },

  setTheme: async (theme) => {
    set({ theme });
    applyTheme(theme);
    await setTheme(theme);
  },

  setEditorFont: async (family, size) => {
    set({ editorFontFamily: family, editorFontSize: size });
    applyEditorFont(family, size);
    await setEditorFont(family, size);
  },
}));
