import { useEffect, useRef, useState } from "react";
import "./EditorPane.css";
import { Toolbar } from "./Toolbar";
import { Editor } from "./Editor";
import { FrontmatterPanel } from "./FrontmatterPanel";
import { Preview } from "../Preview/Preview";
import { DocumentOutline } from "./DocumentOutline";
import { useEditorStore } from "../../stores/editor";
import { clampRange } from "../../lib/layout";

export function EditorPane() {
  const viewMode = useEditorStore((s) => s.viewMode);
  const externalChange = useEditorStore((s) => s.externalChange);
  const reloadFromDisk = useEditorStore((s) => s.reloadFromDisk);
  const keepMine = useEditorStore((s) => s.keepMine);
  const body = useEditorStore((s) => s.body);
  const [splitRatio, setSplitRatio] = useState(50);
  const bodyRef = useRef<HTMLDivElement>(null);
  const splitResizeRef = useRef<{ startX: number; startRatio: number; width: number } | null>(null);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => {
      const resize = splitResizeRef.current;
      if (!resize) return;
      const deltaRatio = ((event.clientX - resize.startX) / resize.width) * 100;
      setSplitRatio(clampRange(resize.startRatio + deltaRatio, 20, 80));
    };
    const stopResize = () => {
      splitResizeRef.current = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", stopResize);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", stopResize);
    };
  }, []);

  return (
    <div className="editor-pane">
      <Toolbar />
      {externalChange && (
        <div className="editor-pane__banner">
          <span>This file changed outside Foldown.</span>
          <button className="editor-pane__banner-button" onClick={reloadFromDisk}>
            Reload
          </button>
          <button className="editor-pane__banner-button" onClick={keepMine}>
            Keep mine
          </button>
        </div>
      )}
      <FrontmatterPanel />
      <div ref={bodyRef} className="editor-pane__body" data-mode={viewMode}>
        <div className="editor-pane__source-area" style={{ ...(viewMode === "preview" ? { display: "none" } : {}), ...(viewMode === "split" ? { flex: `0 0 ${splitRatio}%` } : {}) }}>
          <DocumentOutline body={body} />
          <div className="editor-pane__source">
            <Editor />
          </div>
        </div>
        {viewMode === "split" && (
          <div
            className="editor-pane__resize-handle"
            role="separator"
            aria-label="Resize editor and preview"
            onPointerDown={(event) => {
              const width = bodyRef.current?.getBoundingClientRect().width ?? 0;
              if (!width) return;
              event.preventDefault();
              splitResizeRef.current = { startX: event.clientX, startRatio: splitRatio, width };
              document.body.style.cursor = "col-resize";
              document.body.style.userSelect = "none";
            }}
          />
        )}
        {viewMode !== "source" && (
          <div className="editor-pane__preview" style={viewMode === "split" ? { flex: `0 0 ${100 - splitRatio}%` } : undefined}>
            <Preview />
          </div>
        )}
      </div>
    </div>
  );
}

export default EditorPane;
