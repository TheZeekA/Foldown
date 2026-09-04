import { load as loadYaml } from "js-yaml";

export interface SplitFrontmatter {
  /** The raw frontmatter block exactly as it appears in the source, including
   * delimiters and any trailing newline — `prefix + body` always reconstructs
   * the original content byte-for-byte. */
  prefix: string;
  body: string;
  data: Record<string, unknown>;
  error: string | null;
}

// Requires an actual closing "---"/"..." line, unlike a naive "starts with
// ---" check — otherwise a document that simply opens with a markdown
// horizontal rule would have its entire body swallowed as "frontmatter".
const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)(?:\r?\n)?(?:---|\.\.\.)\r?\n?/;

/** Splits a markdown file into its YAML frontmatter (if any) and body. Never
 * throws — malformed frontmatter is reported via `error` and the whole file
 * is treated as body so no content is ever lost. */
export function splitFrontmatter(content: string): SplitFrontmatter {
  const match = content.match(FRONTMATTER_RE);
  if (!match) {
    return { prefix: "", body: content, data: {}, error: null };
  }

  try {
    // js-yaml's load() throws on an empty/whitespace-only document instead
    // of returning undefined — an empty frontmatter block is valid, though.
    const parsed = match[1].trim() === "" ? undefined : loadYaml(match[1]);
    const isMapping = parsed !== null && typeof parsed === "object" && !Array.isArray(parsed);
    if (parsed !== undefined && !isMapping) {
      // A bare scalar/array between the delimiters isn't real key/value
      // frontmatter — treat the whole thing as body rather than guess.
      return { prefix: "", body: content, data: {}, error: null };
    }
    const prefix = match[0];
    return {
      prefix,
      body: content.slice(prefix.length),
      data: (parsed as Record<string, unknown>) ?? {},
      error: null,
    };
  } catch (error) {
    return { prefix: "", body: content, data: {}, error: String(error) };
  }
}
