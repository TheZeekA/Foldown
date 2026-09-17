import { describe, expect, it, vi } from "vitest";
import type { PdfExportPayload } from "./pdfExportProtocol";
import { openPdfExportWith } from "./openPdfExport";

const payload: PdfExportPayload = {
  requestId: "request-1",
  body: "# Note",
  openPath: "C:/notes/note.md",
  workspaceRoot: "C:/notes",
  outputPath: "C:/notes/note.pdf",
};

async function flushPromises() {
  for (let index = 0; index < 6; index += 1) await Promise.resolve();
}

describe("openPdfExportWith", () => {
  it("registers readiness before creating a new window and delivers after ready", async () => {
    const calls: string[] = [];
    let ready!: () => void;
    let finish!: (result: { requestId: string; outputPath: string; error: string | null }) => void;
    const unlisten = vi.fn();
    const operation = openPdfExportWith(payload, {
      getExistingWindow: async () => null,
      listenForReady: async (handler) => {
        calls.push("listen");
        ready = handler;
        return unlisten;
      },
      listenForResult: async (handler) => { finish = handler; return () => undefined; },
      createWindow: async () => { calls.push("create"); },
      emitPayload: async () => { calls.push("emit"); },
      setTimer: () => 1,
      clearTimer: vi.fn(),
    });

    await flushPromises();
    expect(calls).toEqual(["listen", "create"]);
    ready();
    await flushPromises();
    finish({ requestId: payload.requestId, outputPath: payload.outputPath, error: null });
    await operation;
    expect(calls).toEqual(["listen", "create", "emit"]);
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("immediately delivers to an existing hidden window and waits for completion", async () => {
    const calls: string[] = [];
    let finish!: (result: { requestId: string; outputPath: string; error: string | null }) => void;
    const operation = openPdfExportWith(payload, {
      getExistingWindow: async () => ({ label: "pdf-export" }),
      listenForReady: async () => { throw new Error("should not listen"); },
      listenForResult: async (handler) => { finish = handler; return () => undefined; },
      createWindow: async () => { throw new Error("should not create"); },
      emitPayload: async () => { calls.push("emit"); },
      setTimer: () => 1,
      clearTimer: vi.fn(),
    });
    await flushPromises();
    finish({ requestId: payload.requestId, outputPath: payload.outputPath, error: null });
    await expect(operation).resolves.toBe(payload.outputPath);

    expect(calls).toEqual(["emit"]);
  });

  it("rejects on readiness timeout and removes the listener", async () => {
    const unlisten = vi.fn();
    let timeout!: () => void;
    const operation = openPdfExportWith(payload, {
      getExistingWindow: async () => null,
      listenForReady: async () => unlisten,
      listenForResult: async () => () => undefined,
      createWindow: async () => undefined,
      emitPayload: async () => undefined,
      setTimer: (handler) => { timeout = handler; return 1; },
      clearTimer: vi.fn(),
    });

    await flushPromises();
    timeout();
    await expect(operation).rejects.toThrow("timed out");
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("times out even if native window creation never settles", async () => {
    let timeout!: () => void;
    const operation = openPdfExportWith(payload, {
      getExistingWindow: async () => null,
      listenForReady: async () => () => undefined,
      listenForResult: async () => () => undefined,
      createWindow: () => new Promise(() => undefined),
      emitPayload: async () => undefined,
      setTimer: (handler) => { timeout = handler; return 1; },
      clearTimer: vi.fn(),
    });

    await flushPromises();
    timeout();
    await expect(operation).rejects.toThrow("timed out");
  });
});
