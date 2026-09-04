import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export const MD_GUIDE_WINDOW_LABEL = "md-guide";

/** Opens the Markdown cheat sheet in its own window, or focuses it if the
 * user already has one open rather than creating a duplicate. */
export async function openMdGuideWindow(): Promise<void> {
  const existing = await WebviewWindow.getByLabel(MD_GUIDE_WINDOW_LABEL);
  if (existing) {
    await existing.setFocus();
    return;
  }
  const webview = new WebviewWindow(MD_GUIDE_WINDOW_LABEL, {
    url: "/",
    title: "Markdown Cheat Sheet",
    width: 640,
    height: 760,
    minWidth: 420,
    minHeight: 400,
    resizable: true,
  });
  webview.once("tauri://error", (event) => {
    console.error("Could not open the Markdown Cheat Sheet window:", event);
  });
}
