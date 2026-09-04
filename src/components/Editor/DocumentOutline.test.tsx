import { describe, expect, it } from "vitest";
import { extractMarkdownHeadings } from "../../editor/outline";

describe("DocumentOutline data", () => {
  it("provides ordered heading data for the outline", () => {
    expect(extractMarkdownHeadings("# Start\n## Details")).toMatchObject([
      { text: "Start", level: 1, from: 0 },
      { text: "Details", level: 2, from: 8 },
    ]);
  });

  it("provides an empty heading list for a document without headings", () => {
    expect(extractMarkdownHeadings("Plain paragraph")).toEqual([]);
  });
});
