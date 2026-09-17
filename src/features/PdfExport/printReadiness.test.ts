import { describe, expect, it, vi } from "vitest";
import { shouldPrintRequest, waitForPrintResources } from "./printReadiness";

class PendingImage {
  complete = false;
  private listeners = new Map<string, Set<() => void>>();

  addEventListener(name: string, listener: () => void) {
    const listeners = this.listeners.get(name) ?? new Set();
    listeners.add(listener);
    this.listeners.set(name, listeners);
  }

  removeEventListener(name: string, listener: () => void) {
    this.listeners.get(name)?.delete(listener);
  }

  settle(name: "load" | "error") {
    this.complete = true;
    for (const listener of this.listeners.get(name) ?? []) listener();
  }
}

describe("waitForPrintResources", () => {
  it("waits for fonts and has nothing else to wait for when there are no images", async () => {
    let releaseFonts!: () => void;
    const fontsReady = new Promise<void>((resolve) => { releaseFonts = resolve; });
    const finished = vi.fn();
    const waiting = waitForPrintResources({ fonts: { ready: fontsReady }, images: [] }).then(finished);

    await Promise.resolve();
    expect(finished).not.toHaveBeenCalled();
    releaseFonts();
    await waiting;
    expect(finished).toHaveBeenCalledOnce();
  });

  it("settles after pending images either load or fail", async () => {
    const loaded = new PendingImage();
    const failed = new PendingImage();
    const waiting = waitForPrintResources({
      fonts: { ready: Promise.resolve() },
      images: [loaded, failed],
    });
    let settled = false;
    void waiting.then(() => { settled = true; });

    loaded.settle("load");
    await Promise.resolve();
    expect(settled).toBe(false);
    failed.settle("error");
    await waiting;
    expect(settled).toBe(true);
  });
});

describe("shouldPrintRequest", () => {
  it("prints a request only when it differs from the last printed request", () => {
    expect(shouldPrintRequest("request-1", null)).toBe(true);
    expect(shouldPrintRequest("request-1", "request-1")).toBe(false);
    expect(shouldPrintRequest("request-2", "request-1")).toBe(true);
  });
});
