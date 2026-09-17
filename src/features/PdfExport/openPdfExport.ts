import { emitTo, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  PDF_EXPORT_PAYLOAD_EVENT,
  PDF_EXPORT_READY_EVENT,
  PDF_EXPORT_WINDOW_LABEL,
  type PdfExportPayload,
} from "./pdfExportProtocol";

interface ExistingWindow {
  label: string;
}

interface PdfExportDependencies {
  getExistingWindow(): Promise<ExistingWindow | null>;
  listenForReady(handler: () => void): Promise<() => void>;
  createWindow(): Promise<void>;
  focusWindow(window: ExistingWindow): Promise<void>;
  emitPayload(payload: PdfExportPayload): Promise<void>;
  setTimer(handler: () => void, delayMs: number): unknown;
  clearTimer(timer: unknown): void;
}

const READY_TIMEOUT_MS = 10_000;

export async function openPdfExportWith(
  payload: PdfExportPayload,
  dependencies: PdfExportDependencies,
): Promise<void> {
  const existing = await dependencies.getExistingWindow();
  if (existing) {
    await dependencies.focusWindow(existing);
    await dependencies.emitPayload(payload);
    return;
  }

  let unlisten: (() => void) | null = null;
  let timer: unknown;
  try {
    let resolveReady!: () => void;
    let rejectReady!: (error: Error) => void;
    const ready = new Promise<void>((resolve, reject) => {
      resolveReady = resolve;
      rejectReady = reject;
    });
    unlisten = await dependencies.listenForReady(resolveReady);
    timer = dependencies.setTimer(
      () => rejectReady(new Error("PDF export window readiness timed out")),
      READY_TIMEOUT_MS,
    );
    await Promise.all([dependencies.createWindow(), ready]);
    await dependencies.emitPayload(payload);
  } finally {
    if (timer !== undefined) dependencies.clearTimer(timer);
    unlisten?.();
  }
}

function createPdfWindow(): Promise<void> {
  return new Promise((resolve, reject) => {
    const window = new WebviewWindow(PDF_EXPORT_WINDOW_LABEL, {
      url: "/",
      title: "Export PDF",
      width: 860,
      height: 760,
      minWidth: 520,
      minHeight: 400,
      resizable: true,
    });
    window.once("tauri://created", () => resolve());
    window.once("tauri://error", (event) => reject(new Error(String(event.payload))));
  });
}

const productionDependencies: PdfExportDependencies = {
  getExistingWindow: () => WebviewWindow.getByLabel(PDF_EXPORT_WINDOW_LABEL),
  listenForReady: async (handler) => listen(PDF_EXPORT_READY_EVENT, handler),
  createWindow: createPdfWindow,
  focusWindow: async (window) => {
    const actual = await WebviewWindow.getByLabel(window.label);
    if (!actual) throw new Error("PDF export window is no longer available");
    await actual.setFocus();
  },
  emitPayload: (payload) => emitTo(PDF_EXPORT_WINDOW_LABEL, PDF_EXPORT_PAYLOAD_EVENT, payload),
  setTimer: (handler, delayMs) => setTimeout(handler, delayMs),
  clearTimer: (timer) => clearTimeout(timer as ReturnType<typeof setTimeout>),
};

let pendingExport: Promise<void> | null = null;

export function openPdfExport(payload: PdfExportPayload): Promise<void> {
  if (pendingExport) return pendingExport;
  pendingExport = openPdfExportWith(payload, productionDependencies).finally(() => {
    pendingExport = null;
  });
  return pendingExport;
}
