import { create } from "zustand";
import {
  confirmDelete,
  createFile,
  createFolder,
  deletePath,
  duplicatePath,
  createWorkspace,
  getRecentWorkspaces,
  getTree,
  indexWorkspace,
  movePath,
  openWorkspace,
  openSingleFile,
  pickWorkspaceFolder,
  removeRecentWorkspace,
} from "../lib/tauriApi";
import type { RecentWorkspace, TreeNode } from "../lib/types";
import { baseName, dirName, isSameOrDescendant, joinPath } from "../lib/paths";
import { useEditorStore } from "./editor";

interface Creating {
  parentPath: string;
  type: "file" | "folder";
}

interface WorkspaceState {
  sessionMode: "none" | "workspace" | "single-file";
  path: string | null;
  loading: boolean;
  error: string | null;
  tree: TreeNode[];
  recentWorkspaces: RecentWorkspace[];
  treeLoading: boolean;
  showAllFiles: boolean;
  renamingPath: string | null;
  creating: Creating | null;

  init: () => Promise<void>;
  choose: () => Promise<void>;
  createNew: (parentPath: string, name: string) => Promise<void>;
  removeRecent: (path: string) => Promise<void>;
  openWorkspaceAt: (path: string) => Promise<void>;
  openSingleFileAt: (path: string) => Promise<void>;
  refreshTree: () => Promise<void>;
  toggleShowAllFiles: () => Promise<void>;

  startCreating: (parentPath: string, type: "file" | "folder") => void;
  cancelCreating: () => void;
  confirmCreating: (name: string) => Promise<void>;

  startRenaming: (path: string) => void;
  cancelRenaming: () => void;
  confirmRenaming: (newName: string) => Promise<void>;

  duplicateNode: (path: string) => Promise<void>;
  deleteNode: (path: string, name: string) => Promise<void>;
  moveNode: (sourcePath: string, targetFolderPath: string) => Promise<void>;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  sessionMode: "none",
  path: null,
  loading: true,
  error: null,
  tree: [],
  recentWorkspaces: [],
  treeLoading: false,
  showAllFiles: false,
  renamingPath: null,
  creating: null,

  init: async () => {
    try {
      const recentWorkspaces = await getRecentWorkspaces();
      set({ path: null, sessionMode: "none", recentWorkspaces, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  choose: async () => {
    const picked = await pickWorkspaceFolder();
    if (!picked) return;
    await get().openWorkspaceAt(picked);
  },

  createNew: async (parentPath, name) => {
    try {
      const path = await createWorkspace(parentPath, name);
      await get().openWorkspaceAt(path);
    } catch (error) {
      set({ error: String(error) });
    }
  },

  removeRecent: async (path) => {
    try {
      await removeRecentWorkspace(path);
      set({ recentWorkspaces: await getRecentWorkspaces(), error: null });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  openWorkspaceAt: async (path) => {
    // Flush and clear the OLD workspace's editor state first, while the
    // backend still considers it the active workspace — saveFile would
    // otherwise be rejected as targeting a non-active workspace once
    // openWorkspace below switches the backend's pointer. If the flush
    // fails, abort: switching workspaces now would strand the unsaved edit
    // with no way back.
    try {
      await useEditorStore.getState().resetForWorkspace();
    } catch (error) {
      set({ error: `Could not save your changes, so the workspace wasn't switched: ${String(error)}` });
      return;
    }
    try {
      const canonicalPath = await openWorkspace(path);
      set({ path: canonicalPath, sessionMode: "workspace", error: null, recentWorkspaces: await getRecentWorkspaces() });
      await get().refreshTree();
      indexWorkspace(path).catch(() => {
        // search is best-effort; the sidebar tree still works without an index
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  openSingleFileAt: async (path) => {
    try {
      await useEditorStore.getState().resetForWorkspace();
    } catch (error) {
      set({ error: `Could not save your changes, so the file wasn't opened: ${String(error)}` });
      return;
    }
    try {
      const session = await openSingleFile(path);
      set({
        path: session.root,
        sessionMode: "single-file",
        tree: [{ type: "file", name: baseName(session.path), path: session.path }],
        treeLoading: false,
        error: null,
      });
      await useEditorStore.getState().openFile(session.path, session.root);
    } catch (error) {
      set({ error: String(error) });
    }
  },

  refreshTree: async () => {
    const { path, showAllFiles, sessionMode } = get();
    if (!path) return;
    if (sessionMode === "single-file") return;
    set({ treeLoading: true });
    try {
      const tree = await getTree(path, showAllFiles);
      set({ tree, treeLoading: false, error: null });
    } catch (error) {
      set({ error: String(error), treeLoading: false });
    }
  },

  toggleShowAllFiles: async () => {
    set((s) => ({ showAllFiles: !s.showAllFiles }));
    await get().refreshTree();
  },

  startCreating: (parentPath, type) => set({ creating: { parentPath, type } }),
  cancelCreating: () => set({ creating: null }),

  confirmCreating: async (name) => {
    const { creating, path } = get();
    if (!creating || !path) return;
    const trimmed = name.trim();
    if (!trimmed) {
      set({ creating: null });
      return;
    }
    const finalName =
      creating.type === "file" && !trimmed.toLowerCase().endsWith(".md")
        ? `${trimmed}.md`
        : trimmed;
    const target = joinPath(creating.parentPath, finalName);
    try {
      if (creating.type === "file") {
        await createFile(target, path);
      } else {
        await createFolder(target, path);
      }
      set({ creating: null });
      await get().refreshTree();
    } catch (error) {
      set({ error: String(error), creating: null });
    }
  },

  startRenaming: (path) => set({ renamingPath: path }),
  cancelRenaming: () => set({ renamingPath: null }),

  confirmRenaming: async (newName) => {
    const { renamingPath, path } = get();
    if (!renamingPath || !path) return;
    const trimmed = newName.trim();
    const currentName = baseName(renamingPath);
    if (!trimmed || trimmed === currentName) {
      set({ renamingPath: null });
      return;
    }
    const finalName = /\.md$/i.test(currentName) && !/\.md$/i.test(trimmed)
      ? `${trimmed}.md`
      : trimmed;
    const target = joinPath(dirName(renamingPath), finalName);
    const editor = useEditorStore.getState();
    try {
      if (editor.openPath === renamingPath && editor.dirty) {
        await editor.saveNow();
      }
      await movePath(renamingPath, target, path);
      set({ renamingPath: null });
      if (editor.openPath === renamingPath) {
        await editor.openFile(target, path);
      }
      await get().refreshTree();
    } catch (error) {
      set({ error: String(error), renamingPath: null });
    }
  },

  duplicateNode: async (path) => {
    const { path: workspacePath } = get();
    if (!workspacePath) return;
    try {
      await duplicatePath(path, workspacePath);
      await get().refreshTree();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  deleteNode: async (path, name) => {
    const { path: workspacePath } = get();
    if (!workspacePath) return;
    const confirmed = await confirmDelete(name);
    if (!confirmed) return;
    try {
      await deletePath(path, workspacePath);
      await get().refreshTree();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  moveNode: async (sourcePath, targetFolderPath) => {
    const { path: workspacePath } = get();
    if (!workspacePath) return;
    if (isSameOrDescendant(targetFolderPath, sourcePath)) return;
    if (dirName(sourcePath) === targetFolderPath.replace(/[\\/]+$/, "")) return;

    const target = joinPath(targetFolderPath, baseName(sourcePath));
    try {
      await movePath(sourcePath, target, workspacePath);
      await get().refreshTree();
    } catch (error) {
      set({ error: String(error) });
    }
  },
}));
