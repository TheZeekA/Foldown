import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdateDetails = Pick<Update, "version" | "body" | "date">;

export function formatUpdateDetails(update: UpdateDetails): string {
  const headline = `Version ${update.version} is available.`;
  const notes = update.body?.trim();
  return notes ? `${headline}\n\n${notes}` : headline;
}

export function formatUpdateCheckError(error: unknown, userInitiated: boolean): string | null {
  return userInitiated ? `Could not check for updates: ${String(error)}` : null;
}

export async function checkForUpdate(): Promise<Update | null> {
  return check();
}

export async function installUpdate(update: Update): Promise<void> {
  await update.downloadAndInstall();
}
