import { describe, expect, it } from "vitest";
import type { RecentWorkspace } from "../../lib/types";
import { buildRecentWorkspaceMenu } from "./workspaceMenu";

const workspace = (overrides: Partial<RecentWorkspace>): RecentWorkspace => ({
  path: "C:\\Notes",
  name: "Notes",
  lastOpened: 1,
  available: true,
  ...overrides,
});

describe("buildRecentWorkspaceMenu", () => {
  it("marks the currently open workspace without hiding unavailable recents", () => {
    expect(buildRecentWorkspaceMenu([
      workspace({ path: "C:\\Notes" }),
      workspace({ path: "C:\\Archive", name: "Archive", available: false }),
    ], "c:/notes")).toEqual([
      { workspace: workspace({ path: "C:\\Notes" }), isCurrent: true },
      { workspace: workspace({ path: "C:\\Archive", name: "Archive", available: false }), isCurrent: false },
    ]);
  });
});
