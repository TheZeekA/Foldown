import { useEffect, useState } from "react";
import { getWorkspaceHealth } from "../../lib/tauriApi";
import type { HealthFinding } from "../../lib/types";
import { useEditorStore } from "../../stores/editor";
import { useWorkspaceStore } from "../../stores/workspace";
import { LinksPanel } from "./LinksPanel";
import { TagsPanel } from "./TagsPanel";
import "./InsightsPanel.css";

type Tab = "links" | "tags" | "health";

export function InsightsPanel({ onClose }: { onClose: () => void }) {
  const workspaceRoot = useWorkspaceStore((state) => state.path);
  const activePath = useEditorStore((state) => state.openPath);
  const openFile = useEditorStore((state) => state.openFile);
  const [tab, setTab] = useState<Tab>("links");
  const [findings, setFindings] = useState<HealthFinding[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { if (tab !== "health" || !workspaceRoot) return; let current = true; setFindings(null); void getWorkspaceHealth(workspaceRoot).then((value) => { if (current) setFindings(value); }).catch((reason) => { if (current) setError(String(reason)); }); return () => { current = false; }; }, [tab, workspaceRoot]);
  if (!workspaceRoot) return null;
  const navigate = (path: string) => { void openFile(path, workspaceRoot); };
  return <section className="insights" aria-label="Workspace insights">
    <header className="insights__header"><strong>Workspace insights</strong><button type="button" onClick={onClose} aria-label="Close workspace insights">×</button></header>
    <div className="insights__tabs" role="tablist">{(["links", "tags", "health"] as Tab[]).map((value) => <button key={value} type="button" role="tab" aria-selected={tab === value} className={tab === value ? "insights__tab--active" : ""} onClick={() => setTab(value)}>{value[0].toUpperCase() + value.slice(1)}</button>)}</div>
    <div className="insights__body">
      {tab === "links" && <LinksPanel workspaceRoot={workspaceRoot} activePath={activePath} onOpen={navigate} />}
      {tab === "tags" && <TagsPanel workspaceRoot={workspaceRoot} onOpen={navigate} />}
      {tab === "health" && (error ? <p className="insights__error">{error}</p> : !findings ? <p className="insights__empty">Checking workspace health…</p> : findings.length === 0 ? <p className="insights__empty">No issues found.</p> : <ul className="insights__health-list">{findings.map((finding, index) => <li key={`${finding.path}-${finding.category}-${index}`}><span className={`insights__severity insights__severity--${finding.severity}`}>{finding.severity}</span><strong>{finding.category}</strong><span>{finding.message}</span><small>{finding.path}{finding.target ? ` · ${finding.target}` : ""}</small></li>)}</ul>)}
    </div>
  </section>;
}

