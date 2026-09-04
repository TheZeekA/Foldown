import { describe, expect, it } from "vitest";
import { citationJumpQuery } from "./citations";
import type { AiContextChunk } from "../../lib/types";

const chunk = (overrides: Partial<AiContextChunk>): AiContextChunk => ({
  path: "note.md", heading: "Document", text: "", score: 0, ordinal: 0, ...overrides,
});

describe("citationJumpQuery", () => {
  it("uses a short excerpt of the chunk's own text so it matches a unique location", () => {
    const text = "This is the exact sentence that should be found in the editor when the citation is clicked, even though the section continues on with much more text after this point.";
    expect(citationJumpQuery(chunk({ text }))).toBe(
      "This is the exact sentence that should be found in the editor",
    );
  });

  it("collapses newlines and extra whitespace so the excerpt matches CodeMirror's single-line-indexOf search", () => {
    expect(citationJumpQuery(chunk({ text: "Line one\n\n  Line   two continues right here" }))).toBe(
      "Line one Line two continues right here",
    );
  });

  it("falls back to the heading when the chunk has no usable text", () => {
    expect(citationJumpQuery(chunk({ text: "   ", heading: "Setup" }))).toBe("Setup");
  });
});
