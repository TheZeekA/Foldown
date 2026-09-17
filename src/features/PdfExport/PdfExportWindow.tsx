import { useEffect, useRef, useState } from "react";
import { emitTo, listen } from "@tauri-apps/api/event";
import { markdownToHtml } from "../../components/Preview/markdownToHtml";
import "../../components/Preview/Preview.css";
import "../../styles/theme.css";
import { useSettingsStore } from "../../stores/settings";
import {
  PDF_EXPORT_PAYLOAD_EVENT,
  PDF_EXPORT_READY_EVENT,
  type PdfExportPayload,
} from "./pdfExportProtocol";
import { shouldPrintRequest, waitForPrintResources } from "./printReadiness";
import "./PdfExportWindow.css";

interface RenderedExport {
  requestId: string;
  html: string;
}

export function PdfExportWindow() {
  const initSettings = useSettingsStore((state) => state.init);
  const [payload, setPayload] = useState<PdfExportPayload | null>(null);
  const [rendered, setRendered] = useState<RenderedExport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const lastPrintedRequestId = useRef<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    void (async () => {
      const [removeListener] = await Promise.all([
        listen<PdfExportPayload>(PDF_EXPORT_PAYLOAD_EVENT, (event) => setPayload(event.payload)),
        initSettings(),
      ]);
      if (disposed) {
        removeListener();
        return;
      }
      unlisten = removeListener;
      await emitTo("main", PDF_EXPORT_READY_EVENT);
    })().catch((reason) => {
      if (!disposed) setError(`Could not initialize PDF export: ${String(reason)}`);
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [initSettings]);

  useEffect(() => {
    if (!payload) return;
    let cancelled = false;
    setError(null);
    setRendered(null);
    void markdownToHtml(payload.body, payload)
      .then((html) => {
        if (!cancelled) setRendered({ requestId: payload.requestId, html });
      })
      .catch((reason) => {
        if (!cancelled) setError(`Could not render this document: ${String(reason)}`);
      });
    return () => { cancelled = true; };
  }, [payload]);

  useEffect(() => {
    if (!rendered || !shouldPrintRequest(rendered.requestId, lastPrintedRequestId.current)) return;
    let cancelled = false;
    void (async () => {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      await waitForPrintResources({
        fonts: document.fonts,
        images: document.querySelectorAll<HTMLImageElement>(".preview__body img"),
      });
      if (cancelled || !shouldPrintRequest(rendered.requestId, lastPrintedRequestId.current)) return;
      lastPrintedRequestId.current = rendered.requestId;
      window.print();
    })().catch((reason) => {
      if (!cancelled) setError(`Could not prepare this document for printing: ${String(reason)}`);
    });
    return () => { cancelled = true; };
  }, [rendered]);

  return (
    <main className="pdf-export">
      {!payload && !error && <p className="pdf-export__status">Waiting for the document…</p>}
      {payload && !rendered && !error && <p className="pdf-export__status">Preparing print preview…</p>}
      {error && <p className="pdf-export__status pdf-export__status--error">{error}</p>}
      {rendered && (
        <div className="preview">
          {/* eslint-disable-next-line react/no-danger */}
          <div className="preview__body" dangerouslySetInnerHTML={{ __html: rendered.html }} />
        </div>
      )}
    </main>
  );
}
