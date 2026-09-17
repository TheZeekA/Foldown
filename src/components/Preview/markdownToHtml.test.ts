import { describe, expect, it } from "vitest";
import { markdownToHtml } from "./markdownToHtml";

describe("markdownToHtml", () => {
  it("renders GitHub-flavored Markdown through the preview pipeline", async () => {
    const html = await markdownToHtml("| A | B |\n| - | - |\n| 1 | 2 |", {
      openPath: null,
      workspaceRoot: null,
    });

    expect(html).toContain("<table>");
    expect(html).toContain("<td>1</td>");
  });

  it("rewrites local image sources through the supplied asset converter", async () => {
    const html = await markdownToHtml(
      "![Diagram](../assets/diagram.png)",
      { openPath: "C:/notes/docs/guide.md", workspaceRoot: "C:/notes" },
      (path) => `asset://${path}`,
    );

    expect(html).toContain('src="asset://C:/notes/assets/diagram.png"');
  });
});
