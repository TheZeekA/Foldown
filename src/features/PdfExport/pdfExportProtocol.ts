export interface PdfExportPayload {
  requestId: string;
  body: string;
  openPath: string;
  workspaceRoot: string;
}

export const PDF_EXPORT_WINDOW_LABEL = "pdf-export";
export const PDF_EXPORT_READY_EVENT = "pdf-export-ready";
export const PDF_EXPORT_PAYLOAD_EVENT = "pdf-export-payload";

export function createPdfExportPayload(
  body: string,
  openPath: string,
  workspaceRoot: string,
  createId: () => string = () => crypto.randomUUID(),
): PdfExportPayload {
  return { requestId: createId(), body, openPath, workspaceRoot };
}
