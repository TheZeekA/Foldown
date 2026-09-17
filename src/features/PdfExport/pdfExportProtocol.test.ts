import { describe, expect, it } from "vitest";
import {
  PDF_EXPORT_PAYLOAD_EVENT,
  PDF_EXPORT_READY_EVENT,
  PDF_EXPORT_RESULT_EVENT,
  PDF_EXPORT_WINDOW_LABEL,
  createPdfExportPayload,
} from "./pdfExportProtocol";

describe("PDF export protocol", () => {
  it("uses stable event and window names", () => {
    expect(PDF_EXPORT_WINDOW_LABEL).toBe("pdf-export");
    expect(PDF_EXPORT_READY_EVENT).toBe("pdf-export-ready");
    expect(PDF_EXPORT_PAYLOAD_EVENT).toBe("pdf-export-payload");
    expect(PDF_EXPORT_RESULT_EVENT).toBe("pdf-export-result");
  });

  it("creates a snapshot with a unique request ID", () => {
    const first = createPdfExportPayload("body", "C:/notes/note.md", "C:/notes", "C:/notes/note.pdf", () => "one");
    const second = createPdfExportPayload("body", "C:/notes/note.md", "C:/notes", "C:/notes/note.pdf", () => "two");

    expect(first).toEqual({
      requestId: "one",
      body: "body",
      openPath: "C:/notes/note.md",
      workspaceRoot: "C:/notes",
      outputPath: "C:/notes/note.pdf",
    });
    expect(second.requestId).not.toBe(first.requestId);
  });
});
