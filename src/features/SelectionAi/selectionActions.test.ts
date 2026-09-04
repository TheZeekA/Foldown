import { describe, expect, it } from "vitest";
import { buildSelectionPrompt, SELECTION_AI_ACTIONS } from "./selectionActions";

describe("selection AI actions", () => {
  it("exposes all supported actions", () => {
    expect(SELECTION_AI_ACTIONS.map((action) => action.value)).toEqual([
      "explain", "summarize", "rewrite", "clarify", "checklist", "action-items", "translate",
    ]);
  });

  it("builds a prompt with the selected Markdown", () => {
    expect(buildSelectionPrompt("checklist", "  Ship the release  ")).toContain("Ship the release");
    expect(buildSelectionPrompt("checklist", "  Ship the release  ")).toContain("Markdown checklist");
  });

  it("rejects an empty selection", () => {
    expect(() => buildSelectionPrompt("explain", " \n ")).toThrow("Select some text");
  });
});

