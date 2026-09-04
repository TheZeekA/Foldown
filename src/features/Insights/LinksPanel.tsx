import { useEffect, useState } from "react";
import type { LinkRecord, WorkspaceLinks } from "../../lib/types";
import { getWorkspaceLinks } from "../../lib/tauriApi";

function LinkList({ links, workspaceRoot, onOpen }: { links: LinkRecord[]; workspaceRoot: string; onOpen: (path: string) => void }) {
  if (links.length === 0) return <p className="insights__empty">None found.</p>;
  return <ul className="insights__list">{links.map((link, index) => <li key={`${link.sourcePath}-${link.rawTarget}-${index}`}>
    {link.resolvedPath ? <button type="button" onClick={() => onOpen(link.resolvedPath!)} title={`${workspaceRoot}\\${link.resolvedPath}`}>{link.displayText}<small>{link.sourcePath}</small></button> : <span>{link.rawTarget}<small>{link.status === "ambiguous" ? "Ambiguous target" : "Unresolved target"} · {link.sourcePath}</small></span>}
  </li>)}</ul>;
}

export function LinksPanel({ workspaceRoot, activePath, onOpen }: { workspaceRoot: string; activePath: string | null; onOpen: (path: string) => void }) {
  const [data, setData] = useState<WorkspaceLinks | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    void getWorkspaceLinks(workspaceRoot, activePath).then((value) => { if (current) setData(value); }).catch((reason) => { if (current) setError(String(reason)); });
    return () => { current = false; };
  }, [workspaceRoot, activePath]);
  if (error) return <p className="insights__error">{error}</p>;
  if (!data) return <p className="insights__empty">Scanning links…</p>;
  return <div className="insights__sections">
    <section><h3>Backlinks</h3><LinkList links={data.backlinks} workspaceRoot={workspaceRoot} onOpen={onOpen} /></section>
    <section><h3>Outgoing links</h3><LinkList links={data.outgoing} workspaceRoot={workspaceRoot} onOpen={onOpen} /></section>
    <section><h3>Unresolved links</h3><LinkList links={data.unresolved} workspaceRoot={workspaceRoot} onOpen={onOpen} /></section>
  </div>;
}

