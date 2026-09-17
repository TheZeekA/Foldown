import { useEffect, useRef, useState } from "react";
import { useEditorStore } from "../../stores/editor";
import { useWorkspaceStore } from "../../stores/workspace";
import { markdownToHtml } from "./markdownToHtml";
import "./Preview.css";

export function Preview() {
  const body = useEditorStore((s) => s.body);
  const openPath = useEditorStore((s) => s.openPath);
  const workspaceRoot = useWorkspaceStore((s) => s.path);
  const [html, setHtml] = useState("");
  const firstRenderRef = useRef(true);

  // Re-render immediately (no debounce) the moment a different file is opened.
  useEffect(() => {
    firstRenderRef.current = true;
  }, [openPath]);

  useEffect(() => {
    let cancelled = false;
    const render = () => {
      markdownToHtml(body, { openPath, workspaceRoot }).then((rendered) => {
        if (!cancelled) {
          setHtml(rendered);
        }
      });
    };

    if (firstRenderRef.current) {
      firstRenderRef.current = false;
      render();
      return () => {
        cancelled = true;
      };
    }

    const timeout = setTimeout(render, 150);
    return () => {
      cancelled = true;
      clearTimeout(timeout);
    };
  }, [body, openPath, workspaceRoot]);

  return (
    <div className="preview">
      {/* eslint-disable-next-line react/no-danger */}
      <div className="preview__body" dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  );
}

export default Preview;
