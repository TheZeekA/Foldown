# PDF Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export the current unsaved Markdown body through a dedicated WebView2 print-preview window using the existing preview renderer and native print dialog.

**Architecture:** Extract the preview renderer, define a small cross-window event protocol, and coordinate a listener-before-create ready handshake from the main window. The PDF window renders the transferred snapshot, waits for fonts and images, and invokes `window.print()` once per request.

**Tech Stack:** React 19, TypeScript, Vitest, unified/remark/rehype, Tauri v2, WebView2.

**Spec:** `PDF-Export.md`

## Global Constraints

- Use the native WebView2 print dialog; do not add a save dialog or native PDF dependency.
- Export `body` with unsaved changes and exclude frontmatter.
- Keep the current sanitized Markdown renderer as the single source of truth.
- Preserve local-image asset URL rewriting and active theme behavior.
- Work directly on `develop` because the user declined an isolated worktree.

---

### Task 1: Shared Markdown renderer

**Files:**
- Create: `src/components/Preview/markdownToHtml.ts`
- Create: `src/components/Preview/markdownToHtml.test.ts`
- Modify: `src/components/Preview/Preview.tsx`

**Interfaces:**
- Produces: `markdownToHtml(body, context, toAssetUrl?) => Promise<string>`.

- [ ] Write tests proving GFM output and local-image rewriting through an injected asset converter.
- [ ] Run `npm test -- src/components/Preview/markdownToHtml.test.ts` and confirm failure because the module is absent.
- [ ] Extract the module-scoped unified processor and image rewriting into `markdownToHtml`.
- [ ] Update `Preview.tsx` to call the shared function while retaining its existing debounce/cancellation behavior.
- [ ] Run the focused tests and full `npm test`.

### Task 2: Protocol and print readiness

**Files:**
- Create: `src/features/PdfExport/pdfExportProtocol.ts`
- Create: `src/features/PdfExport/pdfExportProtocol.test.ts`
- Create: `src/features/PdfExport/printReadiness.ts`
- Create: `src/features/PdfExport/printReadiness.test.ts`

**Interfaces:**
- Produces: protocol constants, `createPdfExportPayload`, `waitForPrintResources`, and `shouldPrintRequest`.

- [ ] Write tests for unique payload IDs, font/image settling, and one print per request ID.
- [ ] Run focused tests and confirm missing-module failures.
- [ ] Implement the smallest platform-neutral helpers using injectable document/image shapes.
- [ ] Run focused tests and full `npm test`.

### Task 3: Cross-window export coordinator

**Files:**
- Create: `src/features/PdfExport/openPdfExport.ts`
- Create: `src/features/PdfExport/openPdfExport.test.ts`

**Interfaces:**
- Consumes: protocol constants and `PdfExportPayload`.
- Produces: `openPdfExport(payload, dependencies?) => Promise<void>` with production Tauri dependencies and injectable test dependencies.

- [ ] Write tests proving listener registration precedes window creation, existing-window reuse, targeted payload delivery, timeout rejection, and listener cleanup.
- [ ] Run focused tests and confirm failure.
- [ ] Implement the coordinator with a single in-flight guard and ready timeout.
- [ ] Run focused tests and full `npm test`.

### Task 4: PDF export window and toolbar integration

**Files:**
- Create: `src/features/PdfExport/PdfExportWindow.tsx`
- Create: `src/features/PdfExport/PdfExportWindow.css`
- Modify: `src/components/Editor/Toolbar.tsx`
- Modify: `src/components/Editor/Toolbar.css`
- Modify: `src/main.tsx`

**Interfaces:**
- Consumes: shared renderer, protocol, readiness helpers, editor/workspace snapshots.

- [ ] Add the PDF window component: register payload listener, signal readiness, render, wait for resources, and print once per request.
- [ ] Add print CSS for full-document flow, page margins, colors, wrapping, and practical break avoidance.
- [ ] Route the `pdf-export` label in `main.tsx`.
- [ ] Add the guarded Export PDF toolbar action and error dialog.
- [ ] Run `npm run build` to catch React and Tauri API typing errors.

### Task 5: Capabilities and final verification

**Files:**
- Modify: `src-tauri/capabilities/default.json`
- Create if required: `src-tauri/capabilities/pdf-export.json`
- Modify: `PDF-Export.md` only if implementation reveals a documented mismatch.

- [ ] Give `main` event/window-creation access and `pdf-export` only the core/event access it requires.
- [ ] Run `npm test`.
- [ ] Run `npm run build`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `git diff --check` and inspect `git status --short` plus the final diff.
- [ ] Perform manual native print verification if the Windows GUI runtime is available; otherwise state the remaining manual check explicitly.

