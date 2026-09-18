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

  it("resolves an absolute local path inside the workspace", () => {
    expect(resolveLocalImagePath("C:/workspace/assets/diagram.png", "docs/guide.md", "C:/workspace")).toBe("C:/workspace/assets/diagram.png");
    expect(resolveLocalImagePath("C:\\workspace\\assets\\diagram.png", "docs/guide.md", "C:/workspace")).toBe("C:/workspace/assets/diagram.png");
  });

  it("rejects an absolute local path outside the workspace", () => {
    expect(resolveLocalImagePath("C:/other/secret.png", "docs/guide.md", "C:/workspace")).toBeNull();
  });

  it("rejects a protocol-relative URL rather than treating it as external", () => {
    // Regression test: a protocol-relative value resolves against the
    // current page's own origin, which could otherwise be abused to reach
    // Foldown's local-file-serving asset protocol under the guise of an
    // "external" image.
    expect(resolveLocalImagePath("//asset.localhost/C:/secret.png", "note.md", "C:/workspace")).toBeNull();
  });
});
