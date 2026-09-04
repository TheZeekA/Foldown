import { describe, expect, it } from "vitest";
import { splitFrontmatter } from "./frontmatter";

describe("splitFrontmatter", () => {
  it("returns the whole content as body when there is no frontmatter", () => {
    const content = "# Just a heading\n\nSome text.";
    const result = splitFrontmatter(content);
    expect(result.prefix).toBe("");
    expect(result.body).toBe(content);
    expect(result.data).toEqual({});
    expect(result.error).toBeNull();
  });

  it("parses a simple frontmatter block and splits it from the body", () => {
    const content = "---\ntitle: Hello\ntags:\n  - a\n  - b\n---\n# Body heading\n\nText.";
    const result = splitFrontmatter(content);
    expect(result.data).toEqual({ title: "Hello", tags: ["a", "b"] });
    expect(result.body).toBe("# Body heading\n\nText.");
    expect(result.error).toBeNull();
  });

  it("reconstructs the original content exactly from prefix + body", () => {
    const content = "---\ntitle: Round Trip\n---\n\nBody content here.\n";
    const result = splitFrontmatter(content);
    expect(result.prefix + result.body).toBe(content);
  });

  it("never loses content for an unterminated frontmatter block, however gray-matter chooses to parse it", () => {
    const content = "---\ntitle: Hello\n\n# No closing delimiter";
    const result = splitFrontmatter(content);
    expect(result.prefix + result.body).toBe(content);
  });

  it("reports malformed YAML via error without losing content", () => {
    const content = "---\ntitle: [unclosed\n---\nBody.";
    const result = splitFrontmatter(content);
    expect(result.error).not.toBeNull();
    expect(result.prefix).toBe("");
    expect(result.body).toBe(content);
  });

  it("does not treat a leading markdown horizontal rule as frontmatter", () => {
    // gray-matter will happily parse everything after a bare "---" as a raw
    // YAML scalar with no closing delimiter required — this must not swallow
    // an ordinary document that just starts with a thematic break.
    const content = "---\nSome text that starts with a horizontal rule.\n\nMore text.";
    const result = splitFrontmatter(content);
    expect(result.prefix).toBe("");
    expect(result.body).toBe(content);
    expect(result.data).toEqual({});
  });

  it("handles an empty frontmatter block", () => {
    const content = "---\n---\nBody only.";
    const result = splitFrontmatter(content);
    expect(result.data).toEqual({});
    expect(result.body).toBe("Body only.");
  });
});
