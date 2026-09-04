import { useState } from "react";
import { pickWorkspaceFolder } from "../../lib/tauriApi";
import { useWorkspaceStore } from "../../stores/workspace";
import { BrandMark } from "../BrandMark";
import { workspaceNameError } from "./workspaceName";
import "./WorkspaceWelcome.css";

export function WorkspaceWelcome() {
  const { recentWorkspaces, error, choose, createNew, openWorkspaceAt, removeRecent } = useWorkspaceStore();
  const [creating, setCreating] = useState(false);
  const [parentPath, setParentPath] = useState("");
  const [name, setName] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const validationError = workspaceNameError(name);

  const selectParent = async () => {
    const selected = await pickWorkspaceFolder();
    if (selected) setParentPath(selected);
  };
  const submit = async () => {
    if (!parentPath || validationError) return;
    setSubmitting(true);
    try { await createNew(parentPath, name); } finally { setSubmitting(false); }
  };

  return (
    <main className="workspace-welcome">
      <section className="workspace-welcome__card">
        <header className="workspace-welcome__header">
          <BrandMark size={52} withWordmark />
          <p>Local-first Markdown. Plain files, no lock-in.</p>
        </header>
        {recentWorkspaces.length > 0 && (
          <section className="workspace-welcome__recents">
            <h1>Recent workspaces</h1>
            <div className="workspace-welcome__list">
              {recentWorkspaces.map((workspace) => (
                <div className={`recent-workspace${workspace.available ? "" : " recent-workspace--missing"}`} key={workspace.path}>
                  <button className="recent-workspace__open" disabled={!workspace.available} onClick={() => openWorkspaceAt(workspace.path)}>
                    <span>{workspace.name}</span>
                    <small>{workspace.available ? workspace.path : `Folder not found — ${workspace.path}`}</small>
                  </button>
                  {!workspace.available && (
                    <button className="recent-workspace__remove" onClick={() => removeRecent(workspace.path)} aria-label={`Remove ${workspace.name} from recent workspaces`}>Remove</button>
                  )}
                </div>
              ))}
            </div>
          </section>
        )}
        <div className="workspace-welcome__actions">
          <button className="primary-button" onClick={choose}>Open Existing Workspace</button>
          <button className="secondary-button" onClick={() => setCreating((value) => !value)}>Create New Workspace</button>
        </div>
        {creating && (
          <section className="workspace-welcome__create">
            <label>Parent location
              <div className="workspace-welcome__parent">
                <input value={parentPath} readOnly placeholder="Choose where to create it" />
                <button className="secondary-button" onClick={selectParent}>Choose…</button>
              </div>
            </label>
            <label>Workspace name
              <input value={name} onChange={(event) => setName(event.target.value)} placeholder="Project Notes" autoFocus />
            </label>
            {name && validationError && <p className="workspace-welcome__validation">{validationError}</p>}
            <button className="primary-button" disabled={!parentPath || !!validationError || submitting} onClick={submit}>{submitting ? "Creating…" : "Create Workspace"}</button>
          </section>
        )}
        {error && <p className="error-text">{error}</p>}
      </section>
    </main>
  );
}
