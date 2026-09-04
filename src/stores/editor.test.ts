import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/tauriApi", () => ({
  readFile: vi.fn(),
  saveFile: vi.fn(),
  watchFile: vi.fn().mockResolvedValue(undefined),
  unwatchFile: vi.fn().mockResolvedValue(undefined),
}));

import { readFile, saveFile } from "../lib/tauriApi";
import { useEditorStore } from "./editor";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("editor store", () => {
  beforeEach(() => {
    vi.mocked(readFile).mockReset();
    vi.mocked(saveFile).mockReset();
    useEditorStore.setState({
      openPath: null,
      workspaceRoot: null,
      content: "",
      body: "",
      dirty: false,
      saveStatus: "idle",
      error: null,
      requestSeq: 0,
      reloadToken: 0,
      pendingJumpPosition: null,
    });
  });

  it("saveNow rethrows on failure so callers can abort instead of proceeding", async () => {
    vi.mocked(readFile).mockResolvedValue("hello");
    await useEditorStore.getState().openFile("A.md", "C:\\ws");
    useEditorStore.getState().setBody("edited");

    vi.mocked(saveFile).mockRejectedValue(new Error("disk full"));

    await expect(useEditorStore.getState().saveNow(true)).rejects.toThrow("disk full");
    expect(useEditorStore.getState().saveStatus).toBe("error");
  });

  it("stores and clears a document-position jump request", () => {
    useEditorStore.getState().jumpToPosition(42);
    expect(useEditorStore.getState().pendingJumpPosition).toBe(42);

    useEditorStore.getState().clearPendingJumpPosition();
    expect(useEditorStore.getState().pendingJumpPosition).toBeNull();
  });

  it("a stale in-flight openFile can't resurrect content after a newer one wins", async () => {
    // Regression test: clicking file A, then quickly clicking file B before
    // A's read resolves, must never let A's late resolution overwrite B.
    const fileA = deferred<string>();
    const fileB = deferred<string>();
    vi.mocked(readFile).mockImplementationOnce(() => fileA.promise);

    const openA = useEditorStore.getState().openFile("A.md", "C:\\ws");
    vi.mocked(readFile).mockImplementationOnce(() => fileB.promise);
    const openB = useEditorStore.getState().openFile("B.md", "C:\\ws");

    fileB.resolve("content B");
    await openB;
    // A resolves *after* B has already won.
    fileA.resolve("content A");
    await openA;

    expect(useEditorStore.getState().openPath).toBe("B.md");
    expect(useEditorStore.getState().body).toBe("content B");
  });

  it("openFile aborts the switch if flushing the previous dirty file fails", async () => {
    vi.mocked(readFile).mockResolvedValue("original");
    await useEditorStore.getState().openFile("A.md", "C:\\ws");
    useEditorStore.getState().setBody("unsaved edit");

    vi.mocked(saveFile).mockRejectedValue(new Error("network drive unavailable"));
    vi.mocked(readFile).mockResolvedValue("B content");

    await useEditorStore.getState().openFile("B.md", "C:\\ws");

    expect(useEditorStore.getState().openPath).toBe("A.md");
    expect(useEditorStore.getState().body).toBe("unsaved edit");
    expect(useEditorStore.getState().error).toMatch(/network drive unavailable/);
  });

  it("ignores a delayed watcher event when disk content matches the editor", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(readFile).mockResolvedValue("original");
      await useEditorStore.getState().openFile("A.md", "C:\\ws");
      useEditorStore.getState().setBody("edited");

      vi.mocked(saveFile).mockResolvedValue(undefined);
      await useEditorStore.getState().saveNow();

      // The watcher event arrives after the save's notification debounce window.
      vi.advanceTimersByTime(1001);
      vi.mocked(readFile).mockResolvedValue("edited");
      await useEditorStore.getState().handleFileChangedEvent("A.md");

      expect(useEditorStore.getState().externalChange).toBe(false);
    } finally {
      vi.useRealTimers();
    }
  });

  it("shows the external-change banner when disk content differs", async () => {
    vi.mocked(readFile).mockResolvedValueOnce("original");
    await useEditorStore.getState().openFile("A.md", "C:\\ws");

    vi.mocked(readFile).mockResolvedValue("changed outside Foldown");
    await useEditorStore.getState().handleFileChangedEvent("A.md");

    expect(useEditorStore.getState().externalChange).toBe(true);
  });
});
