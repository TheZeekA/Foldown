export type SettingsPageId = "app" | "ai" | "tools" | "about";

export const SETTINGS_PAGES = [
  { id: "app", label: "App Settings" },
  { id: "ai", label: "AI Settings" },
  { id: "tools", label: "Tools" },
  { id: "about", label: "About" },
] as const;

export const normalizeInitialSettingsPage = (page?: SettingsPageId): SettingsPageId => page ?? "app";
