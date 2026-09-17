import { emitTo, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  PDF_EXPORT_PAYLOAD_EVENT,
  PDF_EXPORT_READY_EVENT,
  PDF_EXPORT_RESULT_EVENT,
  PDF_EXPORT_WINDOW_LABEL,
  type PdfExportPayload,
  type PdfExportResult,
} from "./pdfExportProtocol";

interface ExistingWindow {
  label: string;
}

interface PdfExportDependencies {
  getExistingWindow(): Promise<ExistingWindow | null>;
  listenForReady(handler: () => void): Promise<() => void>;
  listenForResult(handler: (result: PdfExportResult) => void): Promise<() => void>;
  createWindow(): Promise<void>;
  emitPayload(payload: PdfExportPayload): Promise<void>;
  setTimer(handler: () => void, delayMs: number): unknown;
  clearTimer(timer: unknown): void;
}

const EXPORT_TIMEOUT_MS = 30_000;

export async function openPdfExportWith(
  payload: PdfExportPayload,
  dependencies: PdfExportDependencies,
): Promise<string> {
  const cleanups: Array<() => void> = [];
  let timer: unknown;
  try {
    let resolveResult!: (path: string) => void;
    let rejectResult!: (error: Error) => void;
    const result = new Promise<string>((resolve, reject) => {
      resolveResult = resolve;
      rejectResult = reject;
    });
    cleanups.push(await dependencies.listenForResult((event) => {
      if (event.requestId !== payload.requestId) return;
      if (event.error) rejectResult(new Error(event.error));
      else resolveResult(event.outputPath);
    }));
    let rejectTimeout!: (error: Error) => void;
    const timeout = new Promise<never>((_, reject) => { rejectTimeout = reject; });
    timer = dependencies.setTimer(
      () => rejectTimeout(new Error("PDF export timed out")),
      EXPORT_TIMEOUT_MS,
    );

    const workflow = (async () => {
      const existing = await dependencies.getExistingWindow();
      if (!existing) {
        let resolveReady!: () => void;
        const ready = new Promise<void>((resolve) => { resolveReady = resolve; });
        cleanups.push(await dependencies.listenForReady(resolveReady));
        await Promise.all([dependencies.createWindow(), ready]);
      }
      await dependencies.emitPayload(payload);
      return await result;
    })();
    return await Promise.race([workflow, timeout]);
  } finally {
    if (timer !== undefined) dependencies.clearTimer(timer);
    for (const cleanup of cleanups) cleanup();
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
      visible: false,
    });
    window.once("tauri://created", () => resolve());
    window.once("tauri://error", (event) => reject(new Error(String(event.payload))));
  });
}

const productionDependencies: PdfExportDependencies = {
  getExistingWindow: () => WebviewWindow.getByLabel(PDF_EXPORT_WINDOW_LABEL),
  listenForReady: async (handler) => listen(PDF_EXPORT_READY_EVENT, handler),
  listenForResult: async (handler) => listen<PdfExportResult>(PDF_EXPORT_RESULT_EVENT, (event) => handler(event.payload)),
  createWindow: createPdfWindow,
  emitPayload: (payload) => emitTo(PDF_EXPORT_WINDOW_LABEL, PDF_EXPORT_PAYLOAD_EVENT, payload),
  setTimer: (handler, delayMs) => setTimeout(handler, delayMs),
  clearTimer: (timer) => clearTimeout(timer as ReturnType<typeof setTimeout>),
};

let pendingExport: Promise<string> | null = null;

export function openPdfExport(payload: PdfExportPayload): Promise<string> {
  if (pendingExport) return pendingExport;
  pendingExport = openPdfExportWith(payload, productionDependencies).finally(() => {
    pendingExport = null;
  });
  return pendingExport;
}
