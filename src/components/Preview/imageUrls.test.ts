import { describe, expect, it } from "vitest";
import { resolveLocalImagePath } from "./imageUrls";

describe("local preview image paths", () => {
  it("resolves an image relative to a nested markdown file", () => {
    expect(resolveLocalImagePath("../assets/diagram.png", "docs/guide.md", "C:/workspace")).toBe("C:/workspace/assets/diagram.png");
  });

  it("leaves external and escaping paths unresolved", () => {
    expect(resolveLocalImagePath("https://example.com/image.png", "note.md", "C:/workspace")).toBeNull();
    expect(resolveLocalImagePath("../../outside.png", "docs/note.md", "C:/workspace")).toBeNull();
  });

  it("decodes URL-escaped spaces in local asset paths", () => {
    expect(resolveLocalImagePath("assets/Screenshot%202026.png", "note.md", "C:/workspace")).toBe("C:/workspace/assets/Screenshot 2026.png");
  });
});
