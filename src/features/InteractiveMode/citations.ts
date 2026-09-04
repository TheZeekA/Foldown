import type { AiContextChunk } from "../../lib/types";

const EXCERPT_WORDS = 12;

/** A short, whitespace-collapsed excerpt of a citation's own retrieved text —
 * used with the editor's jumpToText, which does a literal substring search.
 * Using the chunk's actual text (rather than its heading) avoids jumping to
 * the wrong occurrence when multiple chunks in a document share one heading. */
export function citationJumpQuery(citation: AiContextChunk): string {
  const collapsed = citation.text.replace(/\s+/g, " ").trim();
  if (!collapsed) return citation.heading;
  return collapsed.split(" ").slice(0, EXCERPT_WORDS).join(" ");
}
