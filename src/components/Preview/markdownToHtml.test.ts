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
      (path) => `http://asset.localhost/${path}`,
    );

    expect(html).toContain('src="http://asset.localhost/C:/notes/assets/diagram.png"');
  });

  it("strips a protocol-relative image source instead of rendering it", async () => {
    // Regression test: a protocol-relative src resolves against the app's
    // own origin at render time, which would otherwise let a malicious
    // document read arbitrary local files through the wildcard-scoped asset
    // protocol under the guise of an "external" image.
    const html = await markdownToHtml(
      "![x](//asset.localhost/C:/Users/victim/secret.png)",
      { openPath: "C:/notes/docs/guide.md", workspaceRoot: "C:/notes" },
    );

    expect(html).not.toContain("asset.localhost");
    expect(html).not.toContain("src=");
  });
});
