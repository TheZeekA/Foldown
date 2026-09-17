# PDF Export Design

## Goal

Add an **Export PDF** action for the currently open Markdown document. The
export must use the editor's current in-memory body, including unsaved edits,
and produce output that is visually consistent with the live preview.

Foldown presents a Save PDF file picker, renders through a hidden WebView2
window, and calls WebView2's native `PrintToPdf` API through Rust. The PDF is
written directly to the selected path without showing the Windows print dialog.

## Constraints

- Windows and Tauri v2 remain the supported runtime.
- No external executable or system dependency is introduced. The existing
  WebView2 runtime performs PDF conversion.
- The frontend `unified` Markdown pipeline remains the rendering source of truth.
- Export uses `body`, not raw `content`, so frontmatter remains excluded.
- Export includes unsaved editor changes.
- Local Markdown images continue to use Tauri asset URLs.
- The current `"system" | "light" | "dark"` theme setting is honored.
- Native print settings prevent a pixel-identical guarantee; the target is
  visual consistency with the preview.

## Existing Architecture

- `src/components/Preview/Preview.tsx` owns the current `unified` pipeline.
- `src/components/Preview/imageUrls.ts` rewrites eligible local image sources.
- `src/components/Preview/Preview.css` contains document presentation.
- `src/styles/theme.css` supplies shared CSS custom properties.
- `src/stores/editor.ts` owns `body` and `openPath`.
- `src/stores/workspace.ts` owns the workspace root.
- `src/lib/mdGuideWindow.ts` demonstrates dynamic secondary-window creation.
- `src/main.tsx` selects a React root by Tauri window label.

## Chosen Approach

Create a hidden PDF webview and call WebView2 `PrintToPdf` through a
Windows-specific Tauri command after rendering settles. The main window uses
`tauri-plugin-dialog` only to select the output path; no print dialog appears.

Rejected alternatives are WebView2 COM interop, WeasyPrint or system CLIs, and
a duplicate Rust Markdown renderer. They add platform complexity, dependencies,
or rendering drift that this version does not need.

## Components

### Shared Markdown renderer

Create `src/components/Preview/markdownToHtml.ts` with:

```ts
export interface MarkdownRenderContext {
  openPath: string | null;
  workspaceRoot: string | null;
}

export async function markdownToHtml(
  body: string,
  context: MarkdownRenderContext,
): Promise<string>;
```

The module owns the existing module-scoped `unified` processor. After rendering,
it calls `replaceLocalImageSources` when both paths exist. `Preview.tsx` and the
PDF window both call this function.

### Export payload and events

Create `src/features/PdfExport/pdfExportProtocol.ts` with serializable types and
constants:

```ts
export interface PdfExportPayload {
  requestId: string;
  body: string;
  openPath: string;
  workspaceRoot: string;
  outputPath: string;
}

export const PDF_EXPORT_WINDOW_LABEL = "pdf-export";
export const PDF_EXPORT_READY_EVENT = "pdf-export-ready";
export const PDF_EXPORT_PAYLOAD_EVENT = "pdf-export-payload";
export const PDF_EXPORT_RESULT_EVENT = "pdf-export-result";
```

The request ID prevents stale payloads from being printed when the window is
reused.

### Main-window coordinator

Create `src/features/PdfExport/openPdfExport.ts`. `openPdfExport(payload)`:

1. Registers result and ready-event listeners before creating the hidden window.
2. Creates the `pdf-export` `WebviewWindow` dynamically when absent.
3. Waits for a ready event from that window.
4. Sends the captured payload with `emitTo(PDF_EXPORT_WINDOW_LABEL, ...)`.
5. Waits for a matching success/failure result from the export window.
6. Removes temporary listeners after completion or failure.
7. Rejects if creation, delivery, conversion, or readiness times out.

The payload is captured when Export is clicked, so later editing cannot alter
an export already in progress. Reusing the window is permitted; each click gets
a new request ID. The toolbar action is disabled without both an open path and
workspace root. Failures are logged and shown with the existing dialog plugin.

### PDF window

Create `src/features/PdfExport/PdfExportWindow.tsx`. On startup it:

1. Registers its payload listener.
2. Emits `PDF_EXPORT_READY_EVENT` to `main` after the listener exists.
3. Initializes theme settings.
4. Renders the latest payload through `markdownToHtml` inside
   `.preview > .preview__body`.
5. Waits for React commit, `document.fonts.ready`, and every current image to
   either load or fail.
6. Invokes `export_webview_to_pdf(outputPath)` once for that request ID.
7. Emits a matching success or failure result to the main window.

The export window is hidden and remains available for later exports. The
one-export-per-request guard must work under React Strict Mode and reset only
for a different request ID.

### Theme and print styling

The PDF window imports `theme.css`, initializes the settings store, and imports
the shared preview CSS. `PdfExportWindow.css` supplies screen status styles and
print rules that:

- set `html`, `body`, `#root`, and `.preview` to automatic height;
- set `.preview` overflow to visible;
- hide screen-only status UI;
- define stable `@page` margins;
- request exact color reproduction;
- avoid breaks immediately after headings and within images, table rows,
  blockquotes, and fenced code blocks when practical;
- wrap long preformatted lines and URLs rather than clipping them.

### Window routing and capabilities

`src/main.tsx` renders `PdfExportWindow` for the `pdf-export` label. The window
is dynamic and is not added to the static `tauri.conf.json` window list.

Update capabilities so both `main` and `pdf-export` have the required event
access. The main window retains webview creation permission. The PDF window gets
no dialog, opener, or process permission. Split the capability files if needed
to keep the PDF window narrowly scoped.

## User Interface

Add an **Export PDF** text action near History and Save Status. It opens a Save
PDF picker defaulting to the Markdown file's name with a `.pdf` extension, then
captures the current store snapshot. It is disabled without an open document,
and repeated clicks are ignored while conversion is pending.

## Error Handling

- Window creation or ready timeout: clear pending state, remove listeners, and
  show a concise dialog.
- Markdown render error: display the error in the PDF window and do not print.
- Image load error: treat the failed image as settled so printing cannot hang.
- Theme initialization error: retain the existing CSS/system fallback.
- Repeated export: accept a new request ID and print it exactly once.

## Files

Create:

- `src/components/Preview/markdownToHtml.ts`
- `src/components/Preview/markdownToHtml.test.ts`
- `src/features/PdfExport/pdfExportProtocol.ts`
- `src/features/PdfExport/pdfExportProtocol.test.ts`
- `src/features/PdfExport/openPdfExport.ts`
- `src/features/PdfExport/openPdfExport.test.ts`
- `src/features/PdfExport/PdfExportWindow.tsx`
- `src/features/PdfExport/PdfExportWindow.css`
- `src/features/PdfExport/printReadiness.ts`
- `src/features/PdfExport/printReadiness.test.ts`
- `src-tauri/src/commands/pdf_export.rs`

Modify:

- `src/components/Preview/Preview.tsx`
- `src/components/Editor/Toolbar.tsx`
- `src/main.tsx`
- `src-tauri/capabilities/default.json`, or split capabilities if required
- `src-tauri/Cargo.toml`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`

The Rust implementation directly depends on the same `webview2-com` version
used by Tauri so it can access `ICoreWebView2_7::PrintToPdf`.

## Testing

Automated tests cover shared rendering and image rewriting; protocol payloads;
resource readiness for no, loaded, and failed images; coordinator ordering,
delivery, reuse, timeout, and cleanup; and one print invocation per request ID.

Verification commands:

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Manual Windows verification uses a multi-page document containing headings,
tables, task lists, blockquotes, long code lines, links, and local images.
Verify all themes, unsaved changes, pagination, repeated exports, Save-dialog
cancellation, and direct PDF output without a print dialog.

## Acceptance Criteria

- Export PDF is available only for an open document.
- The snapshot contains unsaved body edits and excludes frontmatter.
- Preview and export use the same Markdown-to-HTML function.
- Local images resolve in the PDF preview.
- The active theme is represented in the generated PDF.
- Printing starts only after HTML, fonts, and images settle.
- React Strict Mode does not cause duplicate conversion jobs.
- Repeated exports print the newest request exactly once.
- Cancelling the Save dialog leaves the app usable and starts no export.
- No Windows print dialog, external PDF renderer, or redundant renderer is used.
- Frontend tests/build and Rust tests pass.
