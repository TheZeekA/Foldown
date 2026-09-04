import type { RecentWorkspace } from "../../lib/types";

export interface RecentWorkspaceMenuItem {
  workspace: RecentWorkspace;
  isCurrent: boolean;
}

function normalizePath(path: string): string {
  return path.replace(/[\\/]+$/, "").replace(/\\/g, "/").toLowerCase();
}

export function buildRecentWorkspaceMenu(
  workspaces: RecentWorkspace[],
  currentPath: string | null,
): RecentWorkspaceMenuItem[] {
  const normalizedCurrentPath = currentPath ? normalizePath(currentPath) : null;
  return workspaces.map((workspace) => ({
    workspace,
    isCurrent: normalizedCurrentPath === normalizePath(workspace.path),
  }));
}
