import { lazy, Suspense, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { message } from "@tauri-apps/plugin-dialog";
import "./styles/theme.css";
import "./App.css";
import { BrandMark } from "./components/BrandMark";
import { WorkspaceWelcome } from "./components/WorkspaceWelcome/WorkspaceWelcome";
import { Sidebar } from "./components/Sidebar/Sidebar";
import { useWorkspaceStore } from "./stores/workspace";
import { useEditorStore } from "./stores/editor";
import { useSettingsStore } from "./stores/settings";
import { openExternalFile } from "./lib/externalOpen";
import { takePendingOpen, watchWorkspace } from "./lib/tauriApi";
import { InteractiveModePanel } from "./features/InteractiveMode/InteractiveModePanel";
import { useInteractiveModeStore } from "./stores/interactiveMode";

const EditorPane = lazy(() => import("./components/Editor/EditorPane"));
const OPEN_FILE_REQUEST_EVENT = "open-file-request";
const AI_INDEX_STATUS_EVENT = "ai-index-status";

function App() {
  const { path, loading, error, init } = useWorkspaceStore();
  const openPath = useEditorStore((s) => s.openPath);
  const initSettings = useSettingsStore((s) => s.init);
  const aiOpen = useInteractiveModeStore((s) => s.isOpen);
  useEffect(() => {
    (async () => {
      await init();
      await initSettings();
      const pending = await takePendingOpen();
      if (pending) await openExternalFile(pending);
    })();
  }, [init, initSettings]);

  useEffect(() => {
    const unlisten = listen<string>("file-changed", (event) => {
      useEditorStore.getState().handleFileChangedEvent(event.payload);
    });
    const unlistenWorkspace = listen("workspace-changed", () => {
      void useWorkspaceStore.getState().refreshTree();
    });
    const unlistenOpenRequest = listen<string>(OPEN_FILE_REQUEST_EVENT, (event) => {
      openExternalFile(event.payload);
    });
    const unlistenIndexStatus = listen<{ status: string; detail: string | null }>(AI_INDEX_STATUS_EVENT, (event) => {
      useInteractiveModeStore.getState().setIndexStatus(event.payload.status as "indexing" | "ready" | "error", event.payload.detail);
    });
    // Tauri's window-level drag-drop capture is disabled (see tauri.conf.json)
    // so the sidebar's own HTML5 drag-and-drop (moving files between folders)
    // can receive drag events — WebView2 only delivers those to the DOM when
    // the native handler isn't also grabbing them. Dropping an external file
    // from Explorer is handled here as a plain HTML5 drop instead; Tauri's
    // webview still exposes the real filesystem path on the dropped File.
    const handleWindowDragOver = (e: DragEvent) => {
      if (e.dataTransfer?.types.includes("Files")) e.preventDefault();
    };
    const handleWindowDrop = (e: DragEvent) => {
      if (e.defaultPrevented) return;
      const files = e.dataTransfer?.files;
      if (!files || files.length === 0) return;
      e.preventDefault();
      const mdFile = Array.from(files).find((f) => /\.md$/i.test(f.name));
      const mdPath = mdFile ? (mdFile as File & { path?: string }).path : undefined;
      if (mdPath) {
        openExternalFile(mdPath);
      } else {
        message("Only Markdown (.md) files can be dropped into Foldown.", {
          title: "Foldown",
          kind: "warning",
        });
      }
    };
    window.addEventListener("dragover", handleWindowDragOver);
    window.addEventListener("drop", handleWindowDrop);
    const flushOnClose = () => {
      // Fire-and-forget: saveNow() now rejects on failure so callers that need
      // to react can, but there's nothing more to do here — it already records
      // the failure in the store's `error` state.
      useEditorStore.getState().saveNow().catch(() => {});
    };
    window.addEventListener("beforeunload", flushOnClose);
    return () => {
      unlisten.then((fn) => fn());
      unlistenWorkspace.then((fn) => fn());
      unlistenOpenRequest.then((fn) => fn());
      unlistenIndexStatus.then((fn) => fn());
      window.removeEventListener("dragover", handleWindowDragOver);
      window.removeEventListener("drop", handleWindowDrop);
      window.removeEventListener("beforeunload", flushOnClose);
    };
  }, []);

  useEffect(() => {
    useInteractiveModeStore.getState().resetForWorkspace();
    if (path) void watchWorkspace(path).catch(() => {});
  }, [path]);

  if (loading) {
    return (
      <main className="app-shell">
        <BrandMark size={48} />
      </main>
    );
  }

  if (!path) {
    return <WorkspaceWelcome />;
  }

  return (
    <div className="workspace-layout">
      <Sidebar />
      {!aiOpen && (
        <main className={`workspace-main${openPath ? "" : " workspace-main--empty"}`}>
          {openPath ? (
            <Suspense fallback={null}>
              <EditorPane />
            </Suspense>
          ) : (
            <p className="workspace-main__placeholder">Select a file to start editing.</p>
          )}
          {error && <p className="error-text">{error}</p>}
        </main>
      )}
      {aiOpen && <InteractiveModePanel />}
    </div>
  );
}

export default App;
