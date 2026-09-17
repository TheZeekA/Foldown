import { useWorkspaceStore } from "../stores/workspace";
import { useEditorStore } from "../stores/editor";
import { importFile } from "./tauriApi";
import { isSameOrDescendant } from "./paths";

/** Opens a Windows Explorer request as an isolated single-file session. */
export async function openExternalFile(path: string): Promise<void> {
  await useWorkspaceStore.getState().openSingleFileAt(path);
}

/** Preserves workspace import semantics for files dropped onto the app. */
export async function openDroppedFile(path: string): Promise<void> {
  const workspace = useWorkspaceStore.getState();
  const editor = useEditorStore.getState();

  if (!workspace.path) {
    await workspace.openSingleFileAt(path);
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
