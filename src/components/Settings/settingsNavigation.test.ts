import { describe, expect, it } from "vitest";
import { normalizeInitialSettingsPage, SETTINGS_PAGES } from "./settingsNavigation";

describe("settings navigation", () => {
  it("uses the approved labels in their navigation order", () => {
    expect(SETTINGS_PAGES).toEqual([
      { id: "app", label: "App Settings" },
      { id: "ai", label: "AI Settings" },
      { id: "tools", label: "Tools" },
      { id: "about", label: "About" },
    ]);
  });

  it("defaults to the app settings page", () => {
    expect(normalizeInitialSettingsPage()).toBe("app");
  });

  it("keeps a requested AI settings page", () => {
    expect(normalizeInitialSettingsPage("ai")).toBe("ai");
  });
});
