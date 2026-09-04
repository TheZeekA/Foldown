import { beforeEach, describe, expect, it, vi } from "vitest";

const resetEditorForWorkspace = vi.fn<() => Promise<void>>().mockResolvedValue(undefined);
const openFile = vi.fn<(...args: [string, string]) => Promise<void>>().mockResolvedValue(undefined);
const saveNow = vi.fn<() => Promise<void>>().mockResolvedValue(undefined);

vi.mock("../lib/tauriApi", () => ({
  openWorkspace: vi.fn(),
  movePath: vi.fn().mockResolvedValue(undefined),
  getRecentWorkspaces: vi.fn().mockResolvedValue([]),
  getTree: vi.fn().mockResolvedValue([]),
  indexWorkspace: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("./editor", () => ({
  useEditorStore: { getState: () => ({ resetForWorkspace: resetEditorForWorkspace, openPath: "C:\\ws\\old.md", dirty: false, openFile, saveNow }) },
}));

import { movePath, openWorkspace } from "../lib/tauriApi";
import { useWorkspaceStore } from "./workspace";

describe("workspace switching", () => {
  beforeEach(() => {
    vi.mocked(openWorkspace).mockReset();
    vi.mocked(openWorkspace).mockResolvedValue("C:\\New-Workspace");
    vi.mocked(movePath).mockReset();
    vi.mocked(movePath).mockResolvedValue(undefined);
    openFile.mockReset();
    openFile.mockResolvedValue(undefined);
    saveNow.mockReset();
    saveNow.mockResolvedValue(undefined);
    resetEditorForWorkspace.mockReset();
    resetEditorForWorkspace.mockResolvedValue(undefined);
    useWorkspaceStore.setState({ path: null, error: null });
  });

  it("resets the editor before exposing the new workspace", async () => {
    await useWorkspaceStore.getState().openWorkspaceAt("C:\\New-Workspace");

    expect(resetEditorForWorkspace).toHaveBeenCalledOnce();
    expect(useWorkspaceStore.getState().path).toBe("C:\\New-Workspace");
  });

  it("flushes the OLD workspace's dirty file before the backend switches active workspaces", async () => {
    // Regression test: resetForWorkspace's save must run — and must be
    // allowed to complete — while the backend still considers the OLD
    // workspace active, or the save is rejected as targeting a workspace
    // that's no longer active. That means resetForWorkspace must be called
    // (and awaited) strictly before openWorkspace.
    const order: string[] = [];
    resetEditorForWorkspace.mockImplementation(async () => {
      order.push("resetForWorkspace");
    });
    vi.mocked(openWorkspace).mockImplementation(async () => {
      order.push("openWorkspace");
      return "C:\\New-Workspace";
    });

    await useWorkspaceStore.getState().openWorkspaceAt("C:\\New-Workspace");

    expect(order).toEqual(["resetForWorkspace", "openWorkspace"]);
  });

  it("does not switch workspaces if flushing the old one's unsaved edits fails", async () => {
    // Regression test for the silent-data-loss bug: if the old workspace's
    // dirty file can't be saved, switching anyway would strand/lose that
    // edit with no way back. The switch must abort instead.
    resetEditorForWorkspace.mockRejectedValue(new Error("disk full"));

    await useWorkspaceStore.getState().openWorkspaceAt("C:\\New-Workspace");

    expect(openWorkspace).not.toHaveBeenCalled();
    expect(useWorkspaceStore.getState().path).toBeNull();
    expect(useWorkspaceStore.getState().error).toMatch(/disk full/);
  });

  it("reopens the active editor at its new path after renaming its file", async () => {
    useWorkspaceStore.setState({
      path: "C:\\ws",
      renamingPath: "C:\\ws\\old.md",
    });

    await useWorkspaceStore.getState().confirmRenaming("new.md");

    expect(movePath).toHaveBeenCalledWith("C:\\ws\\old.md", "C:\\ws/new.md", "C:\\ws");
    expect(openFile).toHaveBeenCalledWith("C:\\ws/new.md", "C:\\ws");
  });

  it("keeps the Markdown extension when the new name omits it", async () => {
    useWorkspaceStore.setState({
      path: "C:\\ws",
      renamingPath: "C:\\ws\\old.md",
    });

    await useWorkspaceStore.getState().confirmRenaming("new");

    expect(movePath).toHaveBeenCalledWith("C:\\ws\\old.md", "C:\\ws/new.md", "C:\\ws");
  });
});
