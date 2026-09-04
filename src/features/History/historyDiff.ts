export interface DiffLine {
  kind: "same" | "added" | "removed";
  text: string;
}

export function buildHistoryDiff(current: string, previous: string): DiffLine[] {
  const currentLines = current.split(/\r?\n/);
  const previousLines = previous.split(/\r?\n/);
  const output: DiffLine[] = [];
  const length = Math.max(currentLines.length, previousLines.length);
  for (let index = 0; index < length; index += 1) {
    const currentLine = currentLines[index];
    const previousLine = previousLines[index];
    if (currentLine === previousLine && currentLine !== undefined) {
      output.push({ kind: "same", text: currentLine });
    } else {
      if (previousLine !== undefined) output.push({ kind: "removed", text: previousLine });
      if (currentLine !== undefined) output.push({ kind: "added", text: currentLine });
    }
  }
  return output;
}

