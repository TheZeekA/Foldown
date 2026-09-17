import { describe, expect, it, vi } from "vitest";
import type { PdfExportPayload } from "./pdfExportProtocol";
import { openPdfExportWith } from "./openPdfExport";

const payload: PdfExportPayload = {
  requestId: "request-1",
  body: "# Note",
  openPath: "C:/notes/note.md",
  workspaceRoot: "C:/notes",
};

describe("openPdfExportWith", () => {
  it("registers readiness before creating a new window and delivers after ready", async () => {
    const calls: string[] = [];
    let ready!: () => void;
    const unlisten = vi.fn();
    const operation = openPdfExportWith(payload, {
      getExistingWindow: async () => null,
      listenForReady: async (handler) => {
        calls.push("listen");
        ready = handler;
        return unlisten;
      },
      createWindow: async () => { calls.push("create"); },
      focusWindow: async () => { calls.push("focus"); },
      emitPayload: async () => { calls.push("emit"); },
      setTimer: () => 1,
      clearTimer: vi.fn(),
    });

    await Promise.resolve();
    await Promise.resolve();
    expect(calls).toEqual(["listen", "create"]);
    ready();
    await operation;
    expect(calls).toEqual(["listen", "create", "emit"]);
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("focuses and immediately delivers to an existing window", async () => {
    const calls: string[] = [];
    await openPdfExportWith(payload, {
      getExistingWindow: async () => ({ label: "pdf-export" }),
      listenForReady: async () => { throw new Error("should not listen"); },
      createWindow: async () => { throw new Error("should not create"); },
      focusWindow: async () => { calls.push("focus"); },
      emitPayload: async () => { calls.push("emit"); },
      setTimer: () => 1,
      clearTimer: vi.fn(),
    });

    expect(calls).toEqual(["focus", "emit"]);
  });

  it("rejects on readiness timeout and removes the listener", async () => {
    const unlisten = vi.fn();
    let timeout!: () => void;
    const operation = openPdfExportWith(payload, {
      getExistingWindow: async () => null,
      listenForReady: async () => unlisten,
      createWindow: async () => undefined,
      focusWindow: async () => undefined,
      emitPayload: async () => undefined,
      setTimer: (handler) => { timeout = handler; return 1; },
      clearTimer: vi.fn(),
    });

    await Promise.resolve();
    await Promise.resolve();
    timeout();
    await expect(operation).rejects.toThrow("timed out");
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("times out even if native window creation never settles", async () => {
    let timeout!: () => void;
    const operation = openPdfExportWith(payload, {
      getExistingWindow: async () => null,
      listenForReady: async () => () => undefined,
      createWindow: () => new Promise(() => undefined),
      focusWindow: async () => undefined,
      emitPayload: async () => undefined,
      setTimer: (handler) => { timeout = handler; return 1; },
      clearTimer: vi.fn(),
    });

    await Promise.resolve();
    await Promise.resolve();
    timeout();
    await expect(operation).rejects.toThrow("timed out");
  });
});
