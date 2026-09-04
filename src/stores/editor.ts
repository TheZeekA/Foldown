import { create } from "zustand";
import type { EditorView } from "@codemirror/view";
import { readFile, saveFile, unwatchFile, watchFile } from "../lib/tauriApi";
import { splitFrontmatter } from "../editor/frontmatter";

export type ViewMode = "source" | "split" | "preview";
type SaveStatus = "idle" | "saving" | "saved" | "error";

const AUTOSAVE_DELAY_MS = 800;

let autosaveTimer: ReturnType<typeof setTimeout> | null = null;

interface EditorState {
  openPath: string | null;
  workspaceRoot: string | null;
  /** Full file content, including any frontmatter block — the source of truth for saving. */
  content: string;
  /** Content minus the frontmatter block — what the source editor and preview actually show. */
  body: string;
  frontmatterPrefix: string;
  frontmatterData: Record<string, unknown>;
  frontmatterError: string | null;
  loading: boolean;
  error: string | null;
  view: EditorView | null;
  viewMode: ViewMode;
  dirty: boolean;
  saveStatus: SaveStatus;
  externalChange: boolean;
  reloadToken: number;
  /** A search match to scroll to and select once the target file's content is loaded into the view. */
  pendingJump: string | null;
  /** Bumped on every openFile/resetForWorkspace call; lets a superseded in-flight
   * openFile recognize it's stale (e.g. the user clicked another file, or switched
   * workspaces, before its readFile resolved) and skip applying its result. */
  requestSeq: number;

  openFile: (path: string, workspaceRoot: string) => Promise<void>;
  resetForWorkspace: () => Promise<void>;
  setBody: (value: string) => void;
  setView: (view: EditorView | null) => void;
  setViewMode: (mode: ViewMode) => void;
  saveNow: (force?: boolean) => Promise<void>;
  reloadFromDisk: () => Promise<void>;
  keepMine: () => Promise<void>;
  handleFileChangedEvent: (path: string) => Promise<void>;
  jumpToText: (query: string) => void;
  clearPendingJump: () => void;
}

function applyLoadedContent(content: string) {
  const { prefix, body, data, error } = splitFrontmatter(content);
  return {
    content,
    body,
    frontmatterPrefix: prefix,
    frontmatterData: data,
    frontmatterError: error,
  };
}

export const useEditorStore = create<EditorState>((set, get) => ({
  openPath: null,
  workspaceRoot: null,
  content: "",
  body: "",
  frontmatterPrefix: "",
  frontmatterData: {},
  frontmatterError: null,
  loading: false,
  error: null,
  view: null,
  viewMode: "source",
  dirty: false,
  saveStatus: "idle",
  externalChange: false,
  reloadToken: 0,
  pendingJump: null,
  requestSeq: 0,

  openFile: async (path, workspaceRoot) => {
    const previous = get();
    if (previous.openPath === path) return;

    if (previous.openPath && previous.dirty) {
      try {
        await get().saveNow(true);
      } catch (error) {
        set({ error: `Could not save "${previous.openPath}" before switching files: ${String(error)}` });
        return;
      }
    }
    if (previous.openPath) {
      try {
        await unwatchFile();
      } catch {
        // best-effort — a stale watcher isn't harmful
      }
    }

    if (autosaveTimer) {
      clearTimeout(autosaveTimer);
      autosaveTimer = null;
    }

    const seq = get().requestSeq + 1;
    set({ loading: true, error: null, externalChange: false, requestSeq: seq });
    try {
      const content = await readFile(path, workspaceRoot);
      // Superseded by a newer openFile/resetForWorkspace call while this read
      // was in flight (e.g. the user clicked another file, or switched
      // workspaces) — applying this result now would resurrect stale content.
      if (get().requestSeq !== seq) return;
      set({
        openPath: path,
        workspaceRoot,
        ...applyLoadedContent(content),
        loading: false,
        dirty: false,
        saveStatus: "idle",
        reloadToken: get().reloadToken + 1,
      });
      try {
        await watchFile(path, workspaceRoot);
      } catch {
        // watching is best-effort; external-change detection just won't fire
      }
    } catch (error) {
      if (get().requestSeq !== seq) return;
      set({ error: String(error), loading: false });
    }
  },

  resetForWorkspace: async () => {
    const { openPath, dirty } = get();
    // Let a save failure propagate: the caller (openWorkspaceAt) must not
    // proceed to switch workspaces while this file's edits are unflushed,
    // or they're silently lost with no way back.
    if (dirty) await get().saveNow(true);
    if (openPath) {
      try {
        await unwatchFile();
      } catch {
        // best-effort; the next workspace can still be opened safely
      }
    }
    if (autosaveTimer) {
      clearTimeout(autosaveTimer);
      autosaveTimer = null;
    }
    set((s) => ({
      openPath: null,
      workspaceRoot: null,
      content: "",
      body: "",
      frontmatterPrefix: "",
      frontmatterData: {},
      frontmatterError: null,
      loading: false,
      error: null,
      dirty: false,
      saveStatus: "idle",
      externalChange: false,
      pendingJump: null,
      // Invalidate any openFile still in flight so it can't repopulate the
      // editor with the old workspace's content after this reset.
      requestSeq: s.requestSeq + 1,
    }));
  },

  setBody: (value) => {
    const { frontmatterPrefix } = get();
    set({ body: value, content: frontmatterPrefix + value, dirty: true });
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      // Fire-and-forget: a failure is already recorded in `error`/`saveStatus`
      // by saveNow itself; there's no caller here to hand the rejection to.
      get().saveNow().catch(() => {});
    }, AUTOSAVE_DELAY_MS);
  },

  setView: (view) => set({ view }),
  setViewMode: (viewMode) => set({ viewMode }),

  saveNow: async (force = false) => {
    const { openPath, workspaceRoot, content, dirty } = get();
    if (!openPath || !workspaceRoot) return;
    if (!dirty && !force) return;

    if (autosaveTimer) {
      clearTimeout(autosaveTimer);
      autosaveTimer = null;
    }

    set({ saveStatus: "saving" });
    try {
      await saveFile(openPath, workspaceRoot, content);
      set({ dirty: false, saveStatus: "saved" });
    } catch (error) {
      // Rethrow (in addition to recording the error in state) so callers that
      // are about to do something destructive on the assumption the save
      // succeeded — switching files, switching workspaces — can abort instead.
      set({ saveStatus: "error", error: String(error) });
      throw error;
    }
  },

  reloadFromDisk: async () => {
    const { openPath, workspaceRoot } = get();
    if (!openPath || !workspaceRoot) return;
    try {
      const content = await readFile(openPath, workspaceRoot);
      set((s) => ({
        ...applyLoadedContent(content),
        dirty: false,
        saveStatus: "saved",
        externalChange: false,
        reloadToken: s.reloadToken + 1,
      }));
    } catch (error) {
      set({ error: String(error) });
    }
  },

  keepMine: async () => {
    set({ externalChange: false });
    try {
      await get().saveNow(true);
    } catch {
      // Already recorded in `error`/`saveStatus` by saveNow; re-show the
      // banner so the user isn't left thinking their choice was applied.
      set({ externalChange: true });
    }
  },

  handleFileChangedEvent: async (path) => {
    const { openPath, workspaceRoot } = get();
    if (path !== openPath) return;
    if (!workspaceRoot) {
      set({ externalChange: true });
      return;
    }

    try {
      const diskContent = await readFile(path, workspaceRoot);
      const current = get();
      // The event may have been delayed by the OS until after our atomic save.
      // Compare content instead of relying on a timing window, and re-check
      // the active document in case the user switched files while reading.
      if (current.openPath !== path || current.workspaceRoot !== workspaceRoot) return;
      if (diskContent === current.content) return;
    } catch {
      // A missing/unreadable file is still an external change from the
      // editor's perspective, so let the user choose how to resolve it.
    }
    set({ externalChange: true });
  },

  jumpToText: (query) => set({ pendingJump: query }),
  clearPendingJump: () => set({ pendingJump: null }),
}));
