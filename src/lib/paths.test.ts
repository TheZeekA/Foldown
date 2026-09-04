import { describe, expect, it } from "vitest";
import { isSameOrDescendant } from "./paths";

describe("isSameOrDescendant", () => {
  it("accepts a path beneath its parent", () => {
    expect(isSameOrDescendant("C:\\notes\\project\\file.md", "C:\\notes")).toBe(true);
  });

  it("rejects sibling names that only share a string prefix", () => {
    expect(isSameOrDescendant("C:\\notes-archive\\file.md", "C:\\notes")).toBe(false);
  });

  it("still matches when the two sides use different path separators", () => {
    // Regression test: joinPath always builds "/"-style paths, while paths
    // read back from the backend tree are typically "\"-style on Windows —
    // a mismatch here used to make the drag-and-drop cycle guard miss a
    // genuine descendant.
    expect(isSameOrDescendant("C:/notes/project/file.md", "C:\\notes")).toBe(true);
    expect(isSameOrDescendant("C:\\notes\\project\\file.md", "C:/notes")).toBe(true);
  });
});
