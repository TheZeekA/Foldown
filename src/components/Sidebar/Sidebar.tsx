import { useEffect, useRef, useState } from "react";
import "./Sidebar.css";
import { useWorkspaceStore } from "../../stores/workspace";
import { useSearchStore } from "../../stores/search";
import { TreeItem } from "./TreeItem";
import { ContextMenu, type ContextMenuItem } from "./ContextMenu";
import { NameInput } from "./NameInput";
import { SearchPanel } from "./SearchPanel";
import { SettingsModal } from "../Settings/SettingsModal";
import type { SettingsPageId } from "../Settings/settingsNavigation";
import type { TreeNode } from "../../lib/types";
import { useInteractiveModeStore } from "../../stores/interactiveMode";
import { getAiSettings } from "../../lib/tauriApi";
import { activeProviderConfig } from "../../lib/aiProviderConfig";
import { buildRecentWorkspaceMenu } from "./workspaceMenu";
import { clampRange } from "../../lib/layout";

interface MenuState {
  x: number;
  y: number;
  node: TreeNode | null;
}

export function Sidebar() {
  const {
    path,
    recentWorkspaces,
    tree,
    treeLoading,
    showAllFiles,
    toggleShowAllFiles,
    choose,
    openWorkspaceAt,
    removeRecent,
    creating,
    startCreating,
    confirmCreating,
    cancelCreating,
    startRenaming,
    duplicateNode,
    deleteNode,
    moveNode,
  } = useWorkspaceStore();

  const { isOpen: searchOpen, open: openSearch, close: closeSearch } = useSearchStore();
  const aiOpen = useInteractiveModeStore((s) => s.isOpen);
  const openAi = useInteractiveModeStore((s) => s.open);

  const [menu, setMenu] = useState<MenuState | null>(null);
  const [settingsPage, setSettingsPage] = useState<SettingsPageId | null>(null);
  const [noAiConfigured, setNoAiConfigured] = useState(false);
  const [workspaceMenuOpen, setWorkspaceMenuOpen] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(260);
  const sidebarResizeRef = useRef<{ startX: number; startWidth: number } | null>(null);
  const workspaceSwitcherRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => {
      const resize = sidebarResizeRef.current;
      if (!resize) return;
      setSidebarWidth(clampRange(resize.startWidth + event.clientX - resize.startX, 200, 420));
    };
    const stopResize = () => {
      sidebarResizeRef.current = null;
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

  const folderName = path?.split(/[\\/]/).filter(Boolean).pop() ?? "";
  const recentMenuItems = buildRecentWorkspaceMenu(recentWorkspaces, path);

  useEffect(() => {
    if (!workspaceMenuOpen) return;
    const handlePointerDown = (event: PointerEvent) => {
      if (workspaceSwitcherRef.current && !workspaceSwitcherRef.current.contains(event.target as Node)) {
        setWorkspaceMenuOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setWorkspaceMenuOpen(false);
    };
    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [workspaceMenuOpen]);

  const openMenuFor = (e: React.MouseEvent, node: TreeNode | null) => {
    e.preventDefault();
    e.stopPropagation();
    setMenu({ x: e.clientX, y: e.clientY, node });
  };

  const menuItems: ContextMenuItem[] = (() => {
    if (!path) return [];
    const node = menu?.node ?? null;
    const parentForCreate = node?.type === "folder" ? node.path : path;

    const items: ContextMenuItem[] = [
      { label: "New File", onSelect: () => startCreating(parentForCreate, "file") },
      { label: "New Folder", onSelect: () => startCreating(parentForCreate, "folder") },
    ];

    if (node) {
      items.push(
        { label: "Rename", onSelect: () => startRenaming(node.path) },
        { label: "Duplicate", onSelect: () => duplicateNode(node.path) },
        { label: "Delete", onSelect: () => deleteNode(node.path, node.name) },
      );
    }

    return items;
  })();

  return (
    <aside className="sidebar" style={{ width: sidebarWidth }}>
      <div ref={workspaceSwitcherRef} className="sidebar__workspace-switcher">
      <div className="sidebar__header">
        <div className="sidebar__header-left">
          <button
            className="sidebar__icon-button"
            onClick={() => setSettingsPage("app")}
            title="Settings"
            aria-label="Settings"
          >
            <svg width="16" height="16" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
              <line x1="3" y1="5.5" x2="15" y2="5.5" />
              <line x1="3" y1="9" x2="15" y2="9" />
              <line x1="3" y1="12.5" x2="15" y2="12.5" />
            </svg>
          </button>
        </div>
        <div className="sidebar__header-actions">
          <button
            className={`sidebar__icon-button${searchOpen ? " sidebar__icon-button--active" : ""}`}
            onClick={() => (searchOpen ? closeSearch() : openSearch())}
            title="Search workspace"
            aria-pressed={searchOpen}
          >
            <svg width="14" height="14" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
              <circle cx="8" cy="8" r="5.5" />
              <line x1="12.2" y1="12.2" x2="16" y2="16" />
            </svg>
          </button>
          <button
            className={`sidebar__icon-button${showAllFiles ? " sidebar__icon-button--active" : ""}`}
            onClick={toggleShowAllFiles}
            title="Show all files"
            aria-pressed={showAllFiles}
          >
            All
          </button>
          <button
            className="sidebar__icon-button"
            onClick={(e) => openMenuFor(e, null)}
            title="New file or folder"
          >
            +
          </button>
          <button className="sidebar__icon-button" onClick={choose} title="Change workspace">
            ⋯
          </button>
        </div>
      </div>
      <button
        className="sidebar__workspace-name"
        title={path ?? undefined}
        onClick={() => setWorkspaceMenuOpen((open) => !open)}
        aria-label={`Current workspace: ${folderName}`}
        aria-expanded={workspaceMenuOpen}
        aria-haspopup="menu"
      >
        <span className="sidebar__workspace-name-text">{folderName}</span>
        <span className="sidebar__workspace-chevron" aria-hidden="true">⌄</span>
      </button>
      {workspaceMenuOpen && (
        <div className="sidebar__workspace-menu" role="menu" aria-label="Recent workspaces">
          <div className="sidebar__workspace-menu-heading">Recent workspaces</div>
          {recentMenuItems.length > 0 ? recentMenuItems.map(({ workspace, isCurrent }) => (
            <div className={`sidebar__workspace-menu-item${!workspace.available ? " sidebar__workspace-menu-item--missing" : ""}`} key={workspace.path}>
              <button
                className="sidebar__workspace-menu-open"
                disabled={!workspace.available}
                onClick={() => { setWorkspaceMenuOpen(false); void openWorkspaceAt(workspace.path); }}
                role="menuitem"
              >
                <span className="sidebar__workspace-menu-name">
                  {workspace.name}{isCurrent ? "  · Current" : ""}
                </span>
                <small>{workspace.available ? workspace.path : `Folder not found — ${workspace.path}`}</small>
              </button>
              {!workspace.available && (
                <button
                  className="sidebar__workspace-menu-remove"
                  onClick={() => void removeRecent(workspace.path)}
                  aria-label={`Remove ${workspace.name} from recent workspaces`}
                >
                  Remove
                </button>
              )}
            </div>
          )) : (
            <p className="sidebar__workspace-menu-empty">No recent workspaces.</p>
          )}
          <button className="sidebar__workspace-menu-browse" onClick={() => { setWorkspaceMenuOpen(false); void choose(); }} role="menuitem">
            Open another workspace…
          </button>
        </div>
      )}
      </div>
      {settingsPage && <SettingsModal initialPage={settingsPage} onClose={() => setSettingsPage(null)} />}

      <button className={`sidebar__interactive-button${aiOpen ? " sidebar__interactive-button--active" : ""}`} onClick={async () => {
        try {
          const settings = await getAiSettings();
          const active = activeProviderConfig(settings);
          const configured = settings.provider === "local"
            ? active.baseUrl.trim() && active.chatModel.trim()
            : active.chatModel.trim();
          if (!configured) setNoAiConfigured(true);
          else await openAi();
        } catch { setNoAiConfigured(true); }
      }} aria-pressed={aiOpen}>
        <span>✦</span> Interactive Mode
      </button>

      {noAiConfigured && <div className="settings-modal__overlay" onMouseDown={() => setNoAiConfigured(false)}><div className="settings-modal sidebar__ai-warning" role="dialog" aria-modal="true" onMouseDown={(e) => e.stopPropagation()}>
        <h2>No AI Server Configured</h2><p>Add a server endpoint and choose a model before opening Interactive Mode.</p>
        <div><button onClick={() => { setNoAiConfigured(false); setSettingsPage("ai"); }}>Open Settings</button><button onClick={() => setNoAiConfigured(false)}>Cancel</button></div>
      </div></div>}

      {searchOpen ? (
        <SearchPanel />
      ) : (
        <div
          className="sidebar__tree"
          onContextMenu={(e) => openMenuFor(e, null)}
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            e.preventDefault();
            const sourcePath = e.dataTransfer.getData("text/plain");
            if (sourcePath && path) moveNode(sourcePath, path);
          }}
        >
          {treeLoading && tree.length === 0 && <p className="sidebar__empty">Loading…</p>}
          {!treeLoading && tree.length === 0 && !creating && (
            <p className="sidebar__empty">No Markdown files yet.</p>
          )}
          {tree.map((node) => (
            <TreeItem key={node.path} node={node} depth={0} onContextMenu={openMenuFor} />
          ))}
          {creating?.parentPath === path && (
            <div className="tree-item__row" style={{ paddingLeft: "0.5rem" }}>
              <NameInput onConfirm={confirmCreating} onCancel={cancelCreating} />
            </div>
          )}
        </div>
      )}

      {menu && <ContextMenu x={menu.x} y={menu.y} items={menuItems} onClose={() => setMenu(null)} />}
      <div
        className="sidebar__resize-handle"
        role="separator"
        aria-label="Resize file navigation"
        onPointerDown={(event) => {
          event.preventDefault();
          sidebarResizeRef.current = { startX: event.clientX, startWidth: sidebarWidth };
          document.body.style.cursor = "col-resize";
          document.body.style.userSelect = "none";
        }}
      />
    </aside>
  );
}
