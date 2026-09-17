import { beforeEach, describe, expect, it, vi } from "vitest";

const openSingleFileAt = vi.fn<(...args: [string]) => Promise<void>>();
const refreshTree = vi.fn<() => Promise<void>>();
const openFile = vi.fn<(...args: [string, string]) => Promise<void>>();

vi.mock("../stores/workspace", () => ({
  useWorkspaceStore: {
    getState: () => ({
      path: "C:\\workspace",
      openSingleFileAt,
      refreshTree,
    }),
  },
}));

vi.mock("../stores/editor", () => ({
  useEditorStore: { getState: () => ({ openFile }) },
}));

vi.mock("./tauriApi", () => ({
  importFile: vi.fn().mockResolvedValue("C:\\workspace\\outside.md"),
}));

import { importFile } from "./tauriApi";
import { openDroppedFile, openExternalFile } from "./externalOpen";

describe("external file entry points", () => {
  beforeEach(() => vi.clearAllMocks());

  it("opens Explorer requests as an isolated single-file session", async () => {
    await openExternalFile("C:\\other\\note.md");
    expect(openSingleFileAt).toHaveBeenCalledWith("C:\\other\\note.md");
    expect(importFile).not.toHaveBeenCalled();
  });

  it("continues importing an external drop into the active workspace", async () => {
    await openDroppedFile("C:\\other\\outside.md");
    expect(importFile).toHaveBeenCalledWith("C:\\other\\outside.md", "C:\\workspace");
    expect(refreshTree).toHaveBeenCalledOnce();
    expect(openFile).toHaveBeenCalledWith("C:\\workspace\\outside.md", "C:\\workspace");
  });
});
