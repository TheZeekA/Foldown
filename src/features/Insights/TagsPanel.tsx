import { useEffect, useState } from "react";
import { getFilesForTag, getWorkspaceTags } from "../../lib/tauriApi";
import type { TagSummary } from "../../lib/types";

export function TagsPanel({ workspaceRoot, onOpen }: { workspaceRoot: string; onOpen: (path: string) => void }) {
  const [tags, setTags] = useState<TagSummary[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [files, setFiles] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { let current = true; void getWorkspaceTags(workspaceRoot).then((value) => { if (current) setTags(value); }).catch((reason) => { if (current) setError(String(reason)); }); return () => { current = false; }; }, [workspaceRoot]);
  useEffect(() => { if (!selected) { setFiles([]); return; } let current = true; void getFilesForTag(workspaceRoot, selected).then((value) => { if (current) setFiles(value); }).catch((reason) => { if (current) setError(String(reason)); }); return () => { current = false; }; }, [workspaceRoot, selected]);
  if (error) return <p className="insights__error">{error}</p>;
  if (!tags) return <p className="insights__empty">Reading frontmatter…</p>;
  return <div className="insights__sections">
    <section><h3>Tags</h3>{tags.length === 0 ? <p className="insights__empty">No tags found.</p> : <ul className="insights__tag-list">{tags.map((item) => <li key={item.tag}><button type="button" className={selected === item.tag ? "insights__tag--selected" : ""} onClick={() => setSelected(item.tag)}>#{item.tag}<span>{item.count}</span></button></li>)}</ul>}</section>
    {selected && <section><h3>Files tagged #{selected}</h3><ul className="insights__list">{files.map((file) => <li key={file}><button type="button" onClick={() => onOpen(file)}>{file}</button></li>)}</ul></section>}
  </div>;
}

