import { describe, expect, it } from "vitest";
import {
  PDF_EXPORT_PAYLOAD_EVENT,
  PDF_EXPORT_READY_EVENT,
  PDF_EXPORT_WINDOW_LABEL,
  createPdfExportPayload,
} from "./pdfExportProtocol";

describe("PDF export protocol", () => {
  it("uses stable event and window names", () => {
    expect(PDF_EXPORT_WINDOW_LABEL).toBe("pdf-export");
    expect(PDF_EXPORT_READY_EVENT).toBe("pdf-export-ready");
    expect(PDF_EXPORT_PAYLOAD_EVENT).toBe("pdf-export-payload");
  });

  it("creates a snapshot with a unique request ID", () => {
    const first = createPdfExportPayload("body", "C:/notes/note.md", "C:/notes", () => "one");
    const second = createPdfExportPayload("body", "C:/notes/note.md", "C:/notes", () => "two");

    expect(first).toEqual({
      requestId: "one",
      body: "body",
      openPath: "C:/notes/note.md",
      workspaceRoot: "C:/notes",
    });
    expect(second.requestId).not.toBe(first.requestId);
  });
});
