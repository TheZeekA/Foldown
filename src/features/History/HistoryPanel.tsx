import { useEffect, useState } from "react";
import { clearHistory, deleteHistorySnapshot, getHistoryContent, listHistory, restoreHistorySnapshot } from "../../lib/tauriApi";
import type { HistoryEntry } from "../../lib/types";
import { useEditorStore } from "../../stores/editor";
import { useWorkspaceStore } from "../../stores/workspace";
import { buildHistoryDiff } from "./historyDiff";
import "./HistoryPanel.css";

export function HistoryPanel({ onClose }: { onClose: () => void }) {
  const workspaceRoot = useWorkspaceStore((state) => state.path);
  const path = useEditorStore((state) => state.openPath);
  const currentContent = useEditorStore((state) => state.content);
  const reloadFromDisk = useEditorStore((state) => state.reloadFromDisk);
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [selected, setSelected] = useState<{ entry: HistoryEntry; content: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    if (!workspaceRoot || !path) return;
    setLoading(true);
    try {
      setEntries(await listHistory(workspaceRoot, path));
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    setSelected(null);
    void refresh();
    // refresh intentionally follows the active file only.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspaceRoot, path]);

  const selectEntry = async (entry: HistoryEntry) => {
    if (!workspaceRoot || !path) return;
    try {
      const content = await getHistoryContent(entry.id, workspaceRoot, path);
      setSelected({ entry, content });
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  };

  const restore = async () => {
    if (!selected || !workspaceRoot || !path) return;
    if (!window.confirm("Restore this version? The current version will be saved in history first.")) return;
    try {
      await restoreHistorySnapshot(selected.entry.id, workspaceRoot, path, currentContent);
      await reloadFromDisk();
      await refresh();
      setSelected(null);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  };

  const remove = async (entry: HistoryEntry) => {
    if (!window.confirm("Delete this history entry?")) return;
    try {
      await deleteHistorySnapshot(entry.id);
      if (selected?.entry.id === entry.id) setSelected(null);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  };

  const clear = async () => {
    if (!workspaceRoot || !path || !window.confirm("Clear all history for this file?")) return;
    try {
      await clearHistory(workspaceRoot, path);
      setSelected(null);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  };

  if (!path) return null;
  return (
    <aside className="history-panel" aria-label="Version history">
      <header className="history-panel__header">
        <div><strong>Version history</strong><span>{path.split(/[\\/]/).pop()}</span></div>
        <button type="button" onClick={onClose} aria-label="Close version history">×</button>
      </header>
      {error && <p className="history-panel__error">{error}</p>}
      {loading && <p className="history-panel__empty">Loading history…</p>}
      {!loading && entries.length === 0 && <p className="history-panel__empty">No saved versions yet.</p>}
      <div className="history-panel__entries">
        {entries.map((entry) => (
          <div key={entry.id} className={`history-panel__entry${selected?.entry.id === entry.id ? " history-panel__entry--selected" : ""}`}>
            <button type="button" className="history-panel__entry-main" onClick={() => void selectEntry(entry)}>
              <strong>{new Date(entry.createdAt * 1000).toLocaleString()}</strong>
              <span>{entry.byteLength.toLocaleString()} bytes</span>
            </button>
            <button type="button" className="history-panel__delete" onClick={() => void remove(entry)} aria-label="Delete history entry">×</button>
          </div>
        ))}
      </div>
      {entries.length > 0 && <button type="button" className="history-panel__clear" onClick={() => void clear()}>Clear file history</button>}
      {selected && (
        <section className="history-panel__preview">
          <h3>Preview</h3>
          <div className="history-panel__diff">
            {buildHistoryDiff(currentContent, selected.content).map((line, index) => (
              <div key={`${index}-${line.kind}`} className={`history-panel__diff-line history-panel__diff-line--${line.kind}`}>
                <span>{line.kind === "added" ? "+" : line.kind === "removed" ? "−" : " "}</span>{line.text || " "}
              </div>
            ))}
          </div>
          <button type="button" onClick={() => void restore()}>Restore this version</button>
        </section>
      )}
    </aside>
  );
}

