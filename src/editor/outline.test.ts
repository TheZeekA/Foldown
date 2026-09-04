import { describe, expect, it } from "vitest";
import { extractMarkdownHeadings } from "./outline";

describe("extractMarkdownHeadings", () => {
  it("extracts headings in source order with levels and offsets", () => {
    const source = "# One\n\n## Two\n### Three";
    expect(extractMarkdownHeadings(source)).toEqual([
      { text: "One", level: 1, from: 0, line: 0 },
      { text: "Two", level: 2, from: 7, line: 2 },
      { text: "Three", level: 3, from: 14, line: 3 },
    ]);
  });

  it("trims optional closing markers and preserves duplicate titles", () => {
    expect(extractMarkdownHeadings("## Same ##\n## Same")).toEqual([
      { text: "Same", level: 2, from: 0, line: 0 },
      { text: "Same", level: 2, from: 11, line: 1 },
    ]);
  });

  it("ignores headings inside fenced and indented code", () => {
    const source = "```md\n# fenced\n```\n    ## indented\n# real";
    expect(extractMarkdownHeadings(source)).toEqual([
      { text: "real", level: 1, from: 35, line: 4 },
    ]);
  });

  it("supports tilde fences and headings with up to three leading spaces", () => {
    expect(extractMarkdownHeadings("   ### Three\n~~~\n# hidden\n~~~")).toEqual([
      { text: "Three", level: 3, from: 3, line: 0 },
    ]);
  });

  it("returns no headings for empty or marker-only lines", () => {
    expect(extractMarkdownHeadings("")).toEqual([]);
    expect(extractMarkdownHeadings("#\n##   ")).toEqual([]);
  });
});
