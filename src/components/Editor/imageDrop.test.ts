import { describe, expect, it } from "vitest";
import { buildImageMarkdown, buildImageMarkdownForDocument, isSupportedImagePath } from "./imageDrop";

describe("image drop helpers", () => {
  it("recognizes supported image extensions case-insensitively", () => {
    expect(isSupportedImagePath("C:/Pictures/Diagram.PNG")).toBe(true);
    expect(isSupportedImagePath("C:/Pictures/notes.md")).toBe(false);
  });

  it("builds an image reference with a readable alt label", () => {
    expect(buildImageMarkdown("assets/architecture-diagram.png")).toBe("![architecture diagram](assets/architecture-diagram.png)");
  });

  it("wraps filenames containing spaces in angle brackets", () => {
    expect(buildImageMarkdown("assets/Screenshot 2026-09-01 053220.png")).toBe("![Screenshot 2026 09 01 053220](<assets/Screenshot 2026-09-01 053220.png>)");
  });

  it("creates a relative reference when dragging an existing asset from the sidebar", () => {
    expect(buildImageMarkdownForDocument("C:/workspace/assets/logo.png", "C:/workspace/docs/note.md", "C:/workspace")).toBe("![logo](../assets/logo.png)");
  });
});
