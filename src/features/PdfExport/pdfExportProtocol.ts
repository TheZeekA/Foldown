export interface PdfExportPayload {
  requestId: string;
  body: string;
  openPath: string;
  workspaceRoot: string;
  outputPath: string;
}

export interface PdfExportResult {
  requestId: string;
  outputPath: string;
  error: string | null;
}

export const PDF_EXPORT_WINDOW_LABEL = "pdf-export";
export const PDF_EXPORT_READY_EVENT = "pdf-export-ready";
export const PDF_EXPORT_PAYLOAD_EVENT = "pdf-export-payload";
export const PDF_EXPORT_RESULT_EVENT = "pdf-export-result";

export function createPdfExportPayload(
  body: string,
  openPath: string,
  workspaceRoot: string,
  outputPath: string,
  createId: () => string = () => crypto.randomUUID(),
): PdfExportPayload {
  return { requestId: createId(), body, openPath, workspaceRoot, outputPath };
}
