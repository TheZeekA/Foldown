import { useEffect, useRef } from "react";
import type { DisplayMessage } from "../../stores/interactiveMode";
import { ActionProposalCard } from "./ActionProposalCard";
import { splitAssistantResponse } from "./response";
import type { AiContextChunk } from "../../lib/types";

export function MessageList({ messages, sending, onOpenCitation }: { messages: DisplayMessage[]; sending: boolean; onOpenCitation: (citation: AiContextChunk) => void }) {
  const endRef = useRef<HTMLDivElement>(null);

  // Scroll the sentinel into view on every message change (including each
  // streamed delta, since that produces a new `messages` array). It must live
  // *inside* .ai-messages — that's the actual overflow-y:auto container;
  // .ai-panel itself is overflow:hidden, so a sentinel outside .ai-messages
  // has no scrollable ancestor to act on and silently does nothing.
  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages, sending]);

  return <div className="ai-messages" aria-live="polite">
    {messages.map((message) => <article key={message.id} className={`ai-message ai-message--${message.role}`}>
      <div className="ai-message__role">{message.role === "user" ? "You" : "Foldown AI"}</div>
      <div className="ai-message__content">{splitAssistantResponse(message.content).message}</div>
      {!!message.citations?.length && <details className="ai-citations"><summary>{message.citations.length} workspace source{message.citations.length === 1 ? "" : "s"}</summary>{message.citations.map((citation, i) => <button type="button" className="ai-citations__item" key={`${citation.path}-${i}`} onClick={() => onOpenCitation(citation)}><code>{citation.path}</code>{citation.heading !== "Document" && <span>{citation.heading}</span>}</button>)}</details>}
      {message.proposals?.map((proposal) => <ActionProposalCard key={proposal.id} proposal={proposal} />)}
    </article>)}
    {sending && <div className="ai-message ai-message--assistant ai-message--thinking">Thinking…</div>}
    <div ref={endRef} />
  </div>;
}
