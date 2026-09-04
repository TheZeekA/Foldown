import { useState } from "react";
import type { AiActionProposal } from "../../lib/types";
import { useInteractiveModeStore } from "../../stores/interactiveMode";
import { useWorkspaceStore } from "../../stores/workspace";
import { useEditorStore } from "../../stores/editor";

const COPY: Record<AiActionProposal["actionType"], { verb: string; question: string; confirm: string }> = {
  create: { verb: "Create", question: "Create this file?", confirm: "Create" },
  replace: { verb: "Replace", question: "Overwrite this file's contents?", confirm: "Replace" },
  delete: { verb: "Delete", question: "Are you sure?", confirm: "Delete" },
};

export function ActionProposalCard({ proposal }: { proposal: AiActionProposal }) {
  const { approve, reject } = useInteractiveModeStore();
  const [status, setStatus] = useState<"pending" | "applying" | "applied" | "rejected" | "error">("pending");
  const [error, setError] = useState("");
  const copy = COPY[proposal.actionType];
  const runApprove = async () => {
    setStatus("applying");
    try {
      const path = await approve(proposal.id);
      await useWorkspaceStore.getState().refreshTree();
      if (useEditorStore.getState().openPath === path) await useEditorStore.getState().reloadFromDisk();
      setStatus("applied");
    } catch (reason) { setError(String(reason)); setStatus("error"); }
  };
  return <section className="ai-proposal">
    <header><strong>{copy.verb}</strong><code>{proposal.path}</code></header>
    <p className="ai-proposal__question">{copy.question}</p>
    {proposal.actionType === "replace" && <div className="ai-proposal__diff"><pre className="ai-proposal__old">{proposal.oldContent}</pre><pre className="ai-proposal__new">{proposal.newContent}</pre></div>}
    {proposal.actionType === "create" && <pre className="ai-proposal__new">{proposal.newContent}</pre>}
    {proposal.actionType === "delete" && <pre className="ai-proposal__old">{proposal.oldContent}</pre>}
    {status === "pending" && <div className="ai-proposal__actions"><button onClick={runApprove}>{copy.confirm}</button><button onClick={async () => { await reject(proposal.id); setStatus("rejected"); }}>Cancel</button></div>}
    {status !== "pending" && <p className={`ai-proposal__status ai-proposal__status--${status}`}>{status === "error" ? error : status}</p>}
  </section>;
}
