import { useEffect, useRef, useState } from "react";
import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";
import remarkRehype from "remark-rehype";
import rehypeSanitize from "rehype-sanitize";
import rehypeStringify from "rehype-stringify";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useEditorStore } from "../../stores/editor";
import { useWorkspaceStore } from "../../stores/workspace";
import { replaceLocalImageSources } from "./imageUrls";
import "./Preview.css";

const processor = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkRehype)
  .use(rehypeSanitize)
  .use(rehypeStringify);

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
      processor.process(body).then((file) => {
        if (!cancelled) {
          const rendered = String(file);
          setHtml(openPath && workspaceRoot ? replaceLocalImageSources(rendered, openPath, workspaceRoot, convertFileSrc) : rendered);
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
