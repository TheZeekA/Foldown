import { invoke } from "@tauri-apps/api/core";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import type { AiChatMessage, AiChatResult, AiProvider, AiServerProbe, AiSettings, BulkConvertResult, EditorFont, RecentWorkspace, SearchResult, SelectionAiAction, SelectionAiResult, ThemeMode, TreeNode } from "./types";

/** Typed wrappers around every Rust command — the one place the frontend talks to Tauri's invoke(). */

export async function pickWorkspaceFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export function getTree(workspacePath: string, showAllFiles: boolean): Promise<TreeNode[]> {
  return invoke<TreeNode[]>("get_tree", { workspacePath, showAllFiles });
}

export function readFile(path: string, workspaceRoot: string): Promise<string> {
  return invoke<string>("read_file", { path, workspaceRoot });
}

export function saveFile(path: string, workspaceRoot: string, contents: string): Promise<void> {
  return invoke<void>("save_file", { path, workspaceRoot, contents });
}

export function watchFile(path: string, workspaceRoot: string): Promise<void> {
  return invoke<void>("watch_file", { path, workspaceRoot });
}

export function unwatchFile(): Promise<void> {
  return invoke<void>("unwatch_file");
}

export function watchWorkspace(workspaceRoot: string): Promise<void> {
  return invoke<void>("watch_workspace", { workspaceRoot });
}

export function createFile(path: string, workspaceRoot: string): Promise<void> {
  return invoke<void>("create_file", { path, workspaceRoot });
}

export function createFolder(path: string, workspaceRoot: string): Promise<void> {
  return invoke<void>("create_folder", { path, workspaceRoot });
}

export function movePath(oldPath: string, newPath: string, workspaceRoot: string): Promise<void> {
  return invoke<void>("move_path", { oldPath, newPath, workspaceRoot });
}

export function deletePath(path: string, workspaceRoot: string): Promise<void> {
  return invoke<void>("delete_path", { path, workspaceRoot });
}

export function duplicatePath(path: string, workspaceRoot: string): Promise<string> {
  return invoke<string>("duplicate_path", { path, workspaceRoot });
}

export function importFile(sourcePath: string, workspaceRoot: string): Promise<string> {
  return invoke<string>("import_file", { sourcePath, workspaceRoot });
}

export function takePendingOpen(): Promise<string | null> {
  return invoke<string | null>("take_pending_open");
}

export function indexWorkspace(workspaceRoot: string): Promise<void> {
  return invoke<void>("index_workspace", { workspaceRoot });
}

export function searchWorkspace(query: string): Promise<SearchResult[]> {
  return invoke<SearchResult[]>("search_workspace", { query });
}

export function getTheme(): Promise<ThemeMode | null> {
  return invoke<string | null>("get_theme") as Promise<ThemeMode | null>;
}

export function setTheme(theme: ThemeMode): Promise<void> {
  return invoke<void>("set_theme", { theme });
}

export function getEditorFont(): Promise<EditorFont | null> {
  return invoke<EditorFont | null>("get_editor_font");
}

export function setEditorFont(family: string, size: number): Promise<void> {
  return invoke<void>("set_editor_font", { family, size });
}

export function getRecentWorkspaces(): Promise<RecentWorkspace[]> { return invoke<RecentWorkspace[]>("get_recent_workspaces"); }
export function openWorkspace(path: string): Promise<string> { return invoke<string>("open_workspace", { path }); }
export function removeRecentWorkspace(path: string): Promise<void> { return invoke<void>("remove_recent_workspace", { path }); }
export function createWorkspace(parentPath: string, name: string): Promise<string> { return invoke<string>("create_workspace", { parentPath, name }); }

export function getAiSettings(): Promise<AiSettings> {
  return invoke<AiSettings>("get_ai_settings");
}

export function setAiSettings(settings: AiSettings): Promise<void> {
  return invoke<void>("set_ai_settings", { settings });
}

export function sendAiMessage(workspaceRoot: string, requestId: string, messages: AiChatMessage[], activePath: string | null): Promise<AiChatResult> {
  return invoke<AiChatResult>("send_ai_message", { workspaceRoot, requestId, messages, activePath });
}

export function cancelAiRequest(requestId: string): Promise<void> {
  return invoke<void>("cancel_ai_request", { requestId });
}

export function runSelectionAi(
  workspaceRoot: string,
  requestId: string,
  action: SelectionAiAction,
  selectedText: string,
  activePath: string,
): Promise<SelectionAiResult> {
  return invoke<SelectionAiResult>("run_selection_ai", {
    workspaceRoot,
    requestId,
    action,
    selectedText,
    activePath,
  });
}

export function rebuildAiIndex(workspaceRoot: string): Promise<void> {
  return invoke<void>("rebuild_ai_index", { workspaceRoot });
}

export function applyAiProposal(proposalId: string): Promise<string> {
  return invoke<string>("apply_ai_proposal", { proposalId });
}

export function rejectAiProposal(proposalId: string): Promise<void> {
  return invoke<void>("reject_ai_proposal", { proposalId });
}

export function listAiModels(provider: AiProvider, baseUrl: string, apiKey: string | null): Promise<string[]> {
  return invoke<string[]>("list_ai_models", { provider, baseUrl, apiKey });
}

export function probeAiEndpoints(urls: string[], apiKey: string | null): Promise<AiServerProbe[]> {
  return invoke<AiServerProbe[]>("probe_ai_endpoints", { urls, apiKey });
}

const CONVERTIBLE_EXTENSIONS = ["txt", "html", "htm", "csv", "docx"];

export function convertDocument(sourcePath: string, destPath: string): Promise<void> {
  return invoke<void>("convert_document", { sourcePath, destPath });
}

export function bulkConvertDocuments(
  sourcePaths: string[],
  destDir: string,
): Promise<BulkConvertResult[]> {
  return invoke<BulkConvertResult[]>("bulk_convert_documents", { sourcePaths, destDir });
}

export async function pickConvertSource(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    filters: [{ name: "Convertible documents", extensions: CONVERTIBLE_EXTENSIONS }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickConvertSources(): Promise<string[]> {
  const selected = await open({
    multiple: true,
    filters: [{ name: "Convertible documents", extensions: CONVERTIBLE_EXTENSIONS }],
  });
  if (Array.isArray(selected)) return selected;
  return typeof selected === "string" ? [selected] : [];
}

export function pickMarkdownSavePath(defaultName: string): Promise<string | null> {
  return save({
    defaultPath: defaultName,
    filters: [{ name: "Markdown", extensions: ["md"] }],
  });
}

export async function pickDestinationFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export function confirmDelete(name: string): Promise<boolean> {
  return confirm(`Move "${name}" to the Recycle Bin?`, {
    title: "Delete",
    kind: "warning",
  });
}
