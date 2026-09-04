import { useState } from "react";
import { cancelAiRequest, runSelectionAi } from "../../lib/tauriApi";
import type { SelectionAiAction, SelectionAiResult } from "../../lib/types";
import { useEditorStore } from "../../stores/editor";
import { useWorkspaceStore } from "../../stores/workspace";
import { SELECTION_AI_ACTIONS } from "./selectionActions";
import "./SelectionAiPanel.css";

interface Proposal {
  action: SelectionAiAction;
  from: number;
  to: number;
  selectedText: string;
  result: SelectionAiResult;
}

const requestId = () => globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;

export function SelectionAiPanel() {
  const body = useEditorStore((state) => state.body);
  const selectionRange = useEditorStore((state) => state.selectionRange);
  const replaceRange = useEditorStore((state) => state.replaceRange);
  const openPath = useEditorStore((state) => state.openPath);
  const workspaceRoot = useWorkspaceStore((state) => state.path);
  const [action, setAction] = useState<SelectionAiAction>("rewrite");
  const [proposal, setProposal] = useState<Proposal | null>(null);
  const [sending, setSending] = useState(false);
  const [activeRequest, setActiveRequest] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectedText = selectionRange && selectionRange.to > selectionRange.from
    ? body.slice(selectionRange.from, selectionRange.to)
    : "";

  const run = async () => {
    if (!workspaceRoot || !openPath || !selectionRange || !selectedText.trim() || sending) return;
    const captured = { ...selectionRange, selectedText };
    const id = requestId();
    setSending(true);
    setActiveRequest(id);
    setError(null);
    setProposal(null);
    try {
      const result = await runSelectionAi(workspaceRoot, id, action, selectedText, openPath);
      setProposal({ action, from: captured.from, to: captured.to, selectedText: captured.selectedText, result });
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSending(false);
      setActiveRequest(null);
    }
  };

  const accept = () => {
    if (!proposal) return;
    const current = useEditorStore.getState().body.slice(proposal.from, proposal.to);
    if (current !== proposal.selectedText) {
      setError("The selected text changed. Run the AI action again before applying it.");
      setProposal(null);
      return;
    }
    replaceRange(proposal.from, proposal.to, proposal.result.text);
    setProposal(null);
    setError(null);
  };

  return (
    <section className="selection-ai" aria-label="Selection AI tools">
      <div className="selection-ai__controls">
        <span className="selection-ai__label">AI selection</span>
        <select value={action} onChange={(event) => setAction(event.target.value as SelectionAiAction)} disabled={sending}>
          {SELECTION_AI_ACTIONS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
        </select>
        <button type="button" onMouseDown={(event) => event.preventDefault()} onClick={() => void run()} disabled={!selectedText.trim() || sending || !openPath}>
          {sending ? "Working…" : "Run"}
        </button>
        {sending && activeRequest && <button type="button" className="selection-ai__secondary" onClick={() => void cancelAiRequest(activeRequest)}>Cancel</button>}
        {!selectedText.trim() && <span className="selection-ai__hint">Select text in the editor first.</span>}
      </div>
      {error && <p className="selection-ai__error">{error}</p>}
      {proposal && (
        <div className="selection-ai__proposal">
          <div className="selection-ai__proposal-heading">
            <strong>Proposed {SELECTION_AI_ACTIONS.find((item) => item.value === proposal.action)?.label.toLowerCase()}</strong>
            <span>Review before applying</span>
          </div>
          <div className="selection-ai__diff">
            <pre className="selection-ai__old">{proposal.selectedText}</pre>
            <pre className="selection-ai__new">{proposal.result.text}</pre>
          </div>
          {proposal.result.citations.length > 0 && <small>{proposal.result.citations.length} workspace source(s) used</small>}
          <div className="selection-ai__actions">
            <button type="button" onClick={accept}>Apply replacement</button>
            <button type="button" className="selection-ai__secondary" onClick={() => setProposal(null)}>Discard</button>
            <button type="button" className="selection-ai__secondary" onClick={() => { void navigator.clipboard?.writeText(proposal.result.text); }}>Copy</button>
          </div>
        </div>
      )}
    </section>
  );
}

