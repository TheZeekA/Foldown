export interface MarkdownHeading {
  text: string;
  level: number;
  from: number;
  line: number;
}

function isFenceStart(line: string): boolean {
  return /^\s{0,3}(`{3,}|~{3,})/.test(line);
}

export function extractMarkdownHeadings(markdown: string): MarkdownHeading[] {
  const headings: MarkdownHeading[] = [];
  let offset = 0;
  let fenced = false;
  let fenceCharacter = "";
  let fenceLength = 0;

  for (const [lineIndex, line] of markdown.split(/\r?\n/).entries()) {
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
    offset += line.length + 1;
  }

  return headings;
}
