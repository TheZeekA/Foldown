import { useEffect, useRef } from "react";
import { Annotation, EditorState } from "@codemirror/state";
import { EditorView, basicSetup } from "codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { useEditorStore } from "../../stores/editor";
import "./Editor.css";
import { importImageAsset } from "../../lib/tauriApi";
import { buildImageMarkdown, isSupportedImagePath } from "./imageDrop";
import { message } from "@tauri-apps/plugin-dialog";

/** Tags a programmatic full-doc replace (file load/switch/reload) so the update
 * listener below can tell it apart from a real keystroke — otherwise loading a
 * file's content would immediately mark it "dirty" and queue an autosave. */
const programmaticUpdate = Annotation.define<boolean>();

/** Builds a case-insensitive regex that matches `query` against the raw
 * document text even when whitespace inside the query doesn't line up
 * character-for-character with the document's actual whitespace. This
 * matters because `citationJumpQuery` collapses all whitespace (including
 * real line breaks in the source text) down to single spaces, but the
 * document itself keeps its real newlines — a plain `indexOf` would never
 * find a citation excerpt whose first words spanned a line break. Every run
 * of whitespace in the query becomes `\s+` in the regex (which matches
 * newlines too), so such an excerpt is still found as one contiguous match. */
function buildWhitespaceTolerantMatcher(query: string): RegExp | null {
  const words = query.split(/\s+/).filter(Boolean);
  if (words.length === 0) return null;
  const pattern = words.map((word) => word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("\\s+");
  try {
    return new RegExp(pattern, "i");
  } catch {
    return null;
  }
}

const editorTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "var(--editor-font-size, 0.95rem)",
    backgroundColor: "var(--color-bg)",
    color: "var(--color-text)",
  },
  ".cm-scroller": { overflow: "auto" },
  ".cm-content": { fontFamily: "var(--font-mono)", padding: "1rem 0" },
  ".cm-gutters": {
    backgroundColor: "var(--color-bg)",
    color: "var(--color-text-muted)",
    border: "none",
  },
  ".cm-activeLine": { backgroundColor: "var(--color-selection)" },
  ".cm-activeLineGutter": { backgroundColor: "var(--color-selection)" },
  "&.cm-focused": { outline: "none" },
  ".cm-selectionBackground": { backgroundColor: "var(--color-selection) !important" },
});

export function Editor() {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const loadedTokenRef = useRef<number>(-1);

  const body = useEditorStore((s) => s.body);
  const reloadToken = useEditorStore((s) => s.reloadToken);
  const setBody = useEditorStore((s) => s.setBody);
  const setView = useEditorStore((s) => s.setView);
  const setSelectionRange = useEditorStore((s) => s.setSelectionRange);
  const selectionRange = useEditorStore((s) => s.selectionRange);
  const openPath = useEditorStore((s) => s.openPath);
  const workspaceRoot = useEditorStore((s) => s.workspaceRoot);
  const replaceRange = useEditorStore((s) => s.replaceRange);
  const pendingJump = useEditorStore((s) => s.pendingJump);
  const clearPendingJump = useEditorStore((s) => s.clearPendingJump);
  const pendingJumpPosition = useEditorStore((s) => s.pendingJumpPosition);
  const clearPendingJumpPosition = useEditorStore((s) => s.clearPendingJumpPosition);

  useEffect(() => {
    if (!containerRef.current) return;

    const view = new EditorView({
      state: EditorState.create({
        doc: "",
        extensions: [
          basicSetup,
          markdown({ codeLanguages: languages }),
          EditorView.lineWrapping,
          editorTheme,
          EditorView.updateListener.of((update) => {
            const isProgrammatic = update.transactions.some((tr) => tr.annotation(programmaticUpdate));
            if (update.selectionSet || update.docChanged) {
              const range = update.state.selection.main;
              setSelectionRange({ from: range.from, to: range.to });
            }
            if (update.docChanged && !isProgrammatic) {
              setBody(update.state.doc.toString());
            }
          }),
        ],
      }),
      parent: containerRef.current,
    });
    viewRef.current = view;
    // Reset so the sync effect below always repopulates a freshly created view —
    // without this, React StrictMode's dev-mode mount→unmount→remount leaves the
    // ref pointing at a token the *destroyed* view already loaded, so the second
    // (real, persisted) view instance never gets its initial content dispatched.
    loadedTokenRef.current = -1;
    setView(view);

    return () => {
      setView(null);
      view.destroy();
      viewRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // reloadToken bumps on every file open/switch and on an explicit "reload from
  // disk" — both cases mean the buffer must be replaced wholesale, unlike a
  // normal keystroke (which only updates `content`, not `reloadToken`).
  useEffect(() => {
    const view = viewRef.current;
    if (!view || reloadToken === loadedTokenRef.current) return;
    loadedTokenRef.current = reloadToken;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: body },
      annotations: programmaticUpdate.of(true),
    });
  }, [reloadToken, body]);

  // Jump to a search match once its file's content is loaded into the view.
  // Try the full query as a whitespace-tolerant phrase first — the common
  // case, since an FTS snippet or citation excerpt usually corresponds to an
  // actual literal substring modulo whitespace — and only fall back to the
  // first word if the exact phrase isn't found at all (e.g. genuine wording
  // differences between the doc and the FTS-normalized snippet).
  useEffect(() => {
    const view = viewRef.current;
    if (!view || !pendingJump) return;
    const query = pendingJump.trim();
    if (!query) {
      clearPendingJump();
      return;
    }
    const doc = view.state.doc.toString();
    const lowerDoc = doc.toLowerCase();

    let idx = -1;
    let matchLength = query.length;
    const match = buildWhitespaceTolerantMatcher(query)?.exec(doc);
    if (match) {
      idx = match.index;
      matchLength = match[0].length;
    }
    if (idx === -1) {
      const term = query.split(/\s+/)[0] ?? "";
      idx = term ? lowerDoc.indexOf(term.toLowerCase()) : -1;
      matchLength = term.length;
    }

    if (idx !== -1) {
      view.dispatch({
        selection: { anchor: idx, head: idx + matchLength },
        scrollIntoView: true,
      });
      view.focus();
    }
    clearPendingJump();
  }, [pendingJump, body, clearPendingJump]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view || pendingJumpPosition === null) return;
    const position = Math.max(0, Math.min(pendingJumpPosition, view.state.doc.length));
    view.dispatch({ selection: { anchor: position }, scrollIntoView: true });
    view.focus();
    clearPendingJumpPosition();
  }, [pendingJumpPosition, clearPendingJumpPosition]);

  const handleDrop = async (event: React.DragEvent<HTMLDivElement>) => {
    const dropped = Array.from(event.dataTransfer.files) as Array<File & { path?: string }>;
    const image = dropped.find((file) => file.path && isSupportedImagePath(file.path));
    if (!image?.path) return;
    event.preventDefault();
    event.stopPropagation();
    if (!openPath || !workspaceRoot) return;
    try {
      const assetPath = await importImageAsset(image.path, openPath, workspaceRoot);
      const position = selectionRange?.to ?? 0;
      replaceRange(position, position, `${buildImageMarkdown(assetPath)}\n`);
    } catch (error) {
      await message(`Could not import image: ${String(error)}`, { title: "Foldown", kind: "error" });
    }
  };

  return <div ref={containerRef} className="editor" onDrop={handleDrop} />;
}

export default Editor;
