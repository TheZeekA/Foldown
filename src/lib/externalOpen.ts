import { useWorkspaceStore } from "../stores/workspace";
import { useEditorStore } from "../stores/editor";
import { importFile } from "./tauriApi";
import { dirName, isSameOrDescendant } from "./paths";

/**
 * Opens a file that arrived from outside the app's own tree UI — via
 * "Open with Foldown", a second-instance relaunch, or an Explorer
 * drag-and-drop. If no workspace is open yet, the file's own folder becomes
 * the workspace. If a workspace is already open and the file lives outside
 * it, a copy is imported in first (matching how other note apps handle an
 * external file dropped onto an existing vault) rather than switching the
 * user away from what they already have open.
 */
export async function openExternalFile(path: string): Promise<void> {
  const workspace = useWorkspaceStore.getState();
  const editor = useEditorStore.getState();

  if (!workspace.path) {
    const root = dirName(path);
    await workspace.openWorkspaceAt(root);
    await editor.openFile(path, root);
    return;
  }

  if (isSameOrDescendant(path, workspace.path)) {
    await editor.openFile(path, workspace.path);
    return;
  }

  const imported = await importFile(path, workspace.path);
  await workspace.refreshTree();
  await editor.openFile(imported, workspace.path);
}
