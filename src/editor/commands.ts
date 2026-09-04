import { EditorSelection, type ChangeSpec } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

function wrapSelection(view: EditorView, marker: string, placeholder: string) {
  const { state } = view;
  const tr = state.changeByRange((range) => {
    const body = state.sliceDoc(range.from, range.to) || placeholder;
    return {
      changes: { from: range.from, to: range.to, insert: `${marker}${body}${marker}` },
      range: EditorSelection.range(range.from + marker.length, range.from + marker.length + body.length),
    };
  });
  view.dispatch(state.update(tr, { scrollIntoView: true, userEvent: "input" }));
  view.focus();
}

export function toggleBold(view: EditorView) {
  wrapSelection(view, "**", "bold text");
}
export function toggleItalic(view: EditorView) {
  wrapSelection(view, "*", "italic text");
}
export function toggleStrikethrough(view: EditorView) {
  wrapSelection(view, "~~", "strikethrough text");
}
export function toggleInlineCode(view: EditorView) {
  wrapSelection(view, "`", "code");
}

export function cycleHeading(view: EditorView) {
  const { state } = view;
  const tr = state.changeByRange((range) => {
    const line = state.doc.lineAt(range.from);
    const match = line.text.match(/^(#{1,6})\s/);
    let insert: string;
    if (!match) {
      insert = `# ${line.text}`;
    } else if (match[1].length < 6) {
      insert = `${"#".repeat(match[1].length + 1)} ${line.text.slice(match[0].length)}`;
    } else {
      insert = line.text.slice(match[0].length);
    }
    const delta = insert.length - line.text.length;
    return {
      changes: { from: line.from, to: line.to, insert },
      range: EditorSelection.cursor(Math.max(line.from, range.to + delta)),
    };
  });
  view.dispatch(state.update(tr, { scrollIntoView: true, userEvent: "input" }));
  view.focus();
}

/** Toggles a line prefix (list marker, blockquote, ...) across every line the selection spans. */
function toggleLinePrefix(view: EditorView, prefix: string) {
  const { state } = view;
  const tr = state.changeByRange((range) => {
    const startLine = state.doc.lineAt(range.from);
    const endLine = state.doc.lineAt(range.to);
    const lines = [];
    for (let n = startLine.number; n <= endLine.number; n++) lines.push(state.doc.line(n));
    const shouldRemove = lines.every((l) => l.text.startsWith(prefix));

    const changes: ChangeSpec[] = [];
    let firstLineDelta = 0;
    let totalDelta = 0;
    for (const line of lines) {
      const has = line.text.startsWith(prefix);
      let insert = line.text;
      if (shouldRemove && has) insert = line.text.slice(prefix.length);
      else if (!shouldRemove && !has) insert = prefix + line.text;
      if (insert !== line.text) {
        changes.push({ from: line.from, to: line.to, insert });
        const delta = insert.length - line.text.length;
        if (line.number === startLine.number) firstLineDelta = delta;
        totalDelta += delta;
      }
    }
    return {
      changes,
      range: EditorSelection.range(
        Math.max(startLine.from, range.from + firstLineDelta),
        Math.max(startLine.from, range.to + totalDelta),
      ),
    };
  });
  view.dispatch(state.update(tr, { scrollIntoView: true, userEvent: "input" }));
  view.focus();
}

export function toggleBulletList(view: EditorView) {
  toggleLinePrefix(view, "- ");
}
export function toggleTaskList(view: EditorView) {
  toggleLinePrefix(view, "- [ ] ");
}
export function toggleBlockquote(view: EditorView) {
  toggleLinePrefix(view, "> ");
}

export function toggleOrderedList(view: EditorView) {
  const { state } = view;
  const tr = state.changeByRange((range) => {
    const startLine = state.doc.lineAt(range.from);
    const endLine = state.doc.lineAt(range.to);
    const lines = [];
    for (let n = startLine.number; n <= endLine.number; n++) lines.push(state.doc.line(n));
    const allNumbered = lines.every((l) => /^\d+\.\s/.test(l.text));

    const changes: ChangeSpec[] = [];
    let firstLineDelta = 0;
    let totalDelta = 0;
    lines.forEach((line, i) => {
      const stripped = line.text.replace(/^\d+\.\s/, "");
      const insert = allNumbered ? stripped : `${i + 1}. ${stripped}`;
      if (insert !== line.text) {
        changes.push({ from: line.from, to: line.to, insert });
        const delta = insert.length - line.text.length;
        if (line.number === startLine.number) firstLineDelta = delta;
        totalDelta += delta;
      }
    });
    return {
      changes,
      range: EditorSelection.range(
        Math.max(startLine.from, range.from + firstLineDelta),
        Math.max(startLine.from, range.to + totalDelta),
      ),
    };
  });
  view.dispatch(state.update(tr, { scrollIntoView: true, userEvent: "input" }));
  view.focus();
}

export function insertTable(view: EditorView) {
  const { state } = view;
  const template = "\n| Column 1 | Column 2 |\n| --- | --- |\n| Cell | Cell |\n";
  const tr = state.changeByRange((range) => ({
    changes: { from: range.to, to: range.to, insert: template },
    range: EditorSelection.cursor(range.to + template.length),
  }));
  view.dispatch(state.update(tr, { scrollIntoView: true, userEvent: "input" }));
  view.focus();
}

function insertLinkOrImage(view: EditorView, isImage: boolean) {
  const { state } = view;
  const tr = state.changeByRange((range) => {
    const text = state.sliceDoc(range.from, range.to) || (isImage ? "alt text" : "link text");
    const prefix = isImage ? "![" : "[";
    const insert = `${prefix}${text}](url)`;
    const urlStart = prefix.length + text.length + 2;
    return {
      changes: { from: range.from, to: range.to, insert },
      range: EditorSelection.range(range.from + urlStart, range.from + urlStart + 3),
    };
  });
  view.dispatch(state.update(tr, { scrollIntoView: true, userEvent: "input" }));
  view.focus();
}

export function insertLink(view: EditorView) {
  insertLinkOrImage(view, false);
}
export function insertImage(view: EditorView) {
  insertLinkOrImage(view, true);
}
