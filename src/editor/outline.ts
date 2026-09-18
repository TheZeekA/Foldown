export interface MarkdownHeading {
  text: string;
  level: number;
  from: number;
  line: number;
}

export function extractMarkdownHeadings(markdown: string): MarkdownHeading[] {
  const headings: MarkdownHeading[] = [];
  let offset = 0;
  let fenced = false;
  let fenceCharacter = "";
  let fenceLength = 0;

  for (const [lineIndex, rawLine] of markdown.split("\n").entries()) {
    // Splitting on "\n" alone (rather than /\r?\n/) keeps a CRLF line's "\r"
    // in `rawLine`, so `rawLine.length + 1` below always equals the exact
    // number of characters the line consumed in the original string —
    // stripping the "\r" for matching would otherwise undercount every CRLF
    // line's contribution to `offset`, throwing off every heading position
    // that follows one.
    const line = rawLine.endsWith("\r") ? rawLine.slice(0, -1) : rawLine;
    const fence = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (fence) {
      const marker = fence[1];
      if (!fenced) {
        fenced = true;
        fenceCharacter = marker[0];
        fenceLength = marker.length;
      } else if (marker[0] === fenceCharacter && marker.length >= fenceLength) {
        fenced = false;
      }
    } else if (!fenced && !/^\s{4}/.test(line)) {
      const match = line.match(/^( {0,3})(#{1,6})(?:[ \t]+|$)(.*)$/);
      if (match) {
        const text = match[3].replace(/[ \t]+#+[ \t]*$/, "").trim();
        if (text) {
          headings.push({
            text,
            level: match[2].length,
            from: offset + match[1].length,
            line: lineIndex,
          });
        }
      }
    }
    offset += rawLine.length + 1;
  }

  return headings;
}
