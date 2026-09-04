import { useEffect, useRef, useState } from "react";
import { useInteractiveModeStore } from "../../stores/interactiveMode";
import { useWorkspaceStore } from "../../stores/workspace";
import { useEditorStore } from "../../stores/editor";
import { activeProviderConfig } from "../../lib/aiProviderConfig";
import { MessageList } from "./MessageList";
import { citationJumpQuery } from "./citations";
import { clampRange } from "../../lib/layout";
import { BrandMark } from "../../components/BrandMark";
import "./InteractiveModePanel.css";

export function InteractiveModePanel() {
  const path = useWorkspaceStore((s) => s.path);
  const { settings, models, messages, sending, indexing, indexStatus, error, close, newChat, switchModel, send, cancel, rebuildIndex } = useInteractiveModeStore();
  const openFile = useEditorStore((s) => s.openFile);
  const jumpToText = useEditorStore((s) => s.jumpToText);
  const [text, setText] = useState("");
  const [panelWidth, setPanelWidth] = useState(380);
  const resizeRef = useRef<{ startX: number; startWidth: number } | null>(null);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => {
      const resize = resizeRef.current;
      if (!resize) return;
      setPanelWidth(clampRange(resize.startWidth + resize.startX - event.clientX, 300, Math.max(300, Math.min(window.innerWidth * 0.6, 720))));
    };
    const stopResize = () => {
      resizeRef.current = null;
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
  if (!settings) return <aside className="ai-panel"><header className="ai-panel__header"><strong>Interactive Mode</strong><button onClick={close}>×</button></header><p className="ai-messages__empty">Loading connection settings…</p></aside>;
  const chatModel = activeProviderConfig(settings).chatModel;
  const composer = <form className={`ai-composer${messages.length === 0 ? " ai-composer--welcome" : ""}`} onSubmit={(e) => { e.preventDefault(); if (path) { void send(path, text); setText(""); } }}>
    <textarea value={text} onChange={(e) => setText(e.target.value)} placeholder="Ask about or change your Markdown workspace…" onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); e.currentTarget.form?.requestSubmit(); } }} />
    {sending ? <button type="button" onClick={cancel}>Cancel</button> : <button disabled={!text.trim() || !path}>Send</button>}
  </form>;
  return <aside className="ai-panel" style={{ width: panelWidth }} aria-label="Interactive Mode">
    <div
      className="ai-panel__resize-handle"
      role="separator"
      aria-label="Resize AI chat"
      onPointerDown={(event) => {
        event.preventDefault();
        resizeRef.current = { startX: event.clientX, startWidth: panelWidth };
        document.body.style.cursor = "col-resize";
        document.body.style.userSelect = "none";
      }}
    />
    <header className="ai-panel__header"><div><strong>Interactive Mode</strong><span>Private workspace AI</span></div><div className="ai-panel__model"><select aria-label="Active AI model" value={chatModel} onChange={(e) => void switchModel(e.target.value)}>
      {!models.includes(chatModel) && <option value={chatModel}>{chatModel}</option>}{models.map((model) => <option key={model} value={model}>{model}</option>)}
    </select><button className="ai-panel__new-chat" title="Start a new chat" aria-label="Start a new chat" disabled={sending} onClick={newChat}>New chat</button><button title="Close Interactive Mode" onClick={close}>×</button></div></header>
      {messages.length === 0 ? <div className="ai-panel__welcome">
        <BrandMark size={54} withWordmark />
        <strong>Ask about your workspace</strong>
        <p>Foldown retrieves relevant Markdown excerpts locally and sends only that context to your configured model.</p>
        {composer}
        <div className="ai-messages__tips">
          <span>Try asking me to:</span>
          <ul>
            <li>Find a policy or procedure</li>
            <li>Summarise a document</li>
            <li>Compare two workspace files</li>
            <li>Locate a specific section</li>
          </ul>
        </div>
      </div> : <MessageList messages={messages} sending={sending} onOpenCitation={(citation) => { if (path) { close(); void openFile(citation.path, path).then(() => jumpToText(citationJumpQuery(citation))); } }} />}
      {error && <p className="ai-panel__error">{error}</p>}
      <div className="ai-panel__index">
        <button disabled={indexing || !path} onClick={() => path && rebuildIndex(path)}>{indexing ? "Indexing…" : "Rebuild workspace memory"}</button>
        {indexStatus === "indexing" && <span className="ai-panel__index-status">Indexing workspace…</span>}
      </div>
      {messages.length > 0 && <form className="ai-composer" onSubmit={(e) => { e.preventDefault(); if (path) { void send(path, text); setText(""); } }}>
        <textarea value={text} onChange={(e) => setText(e.target.value)} placeholder="Ask about or change your Markdown workspace…" onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); e.currentTarget.form?.requestSubmit(); } }} />
        {sending ? <button type="button" onClick={cancel}>Cancel</button> : <button disabled={!text.trim() || !path}>Send</button>}
      </form>}
  </aside>;
}
