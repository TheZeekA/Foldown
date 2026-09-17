# Direct PDF Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Windows print popup with direct WebView2 PDF generation to a path selected inside Foldown.

**Architecture:** Add the output path and completion result to the existing cross-window protocol. After the hidden export webview settles, it invokes a Windows-only Rust command that obtains the underlying WebView2 controller and calls `ICoreWebView2_7::PrintToPdf`.

**Tech Stack:** React, TypeScript, Tauri v2, Rust, webview2-com 0.38.2, WebView2.

**Spec:** `PDF-Export.md`

## Global Constraints

- Do not show `window.print()` or the Windows print dialog.
- Keep Chromium rendering, local assets, themes, and unsaved snapshot behavior.
- Validate the output path and report asynchronous WebView2 completion.

---

### Task 1: Extend the frontend protocol

**Files:** `src/features/PdfExport/pdfExportProtocol.ts`, its tests, `openPdfExport.ts`, and its tests.

- [ ] Write failing tests for output paths, matching completion results, errors, and listener cleanup.
- [ ] Implement the extended payload/result protocol and coordinator.
- [ ] Run focused and full frontend tests.

### Task 2: Add native WebView2 PDF command

**Files:** `src-tauri/src/commands/pdf_export.rs`, `commands/mod.rs`, `lib.rs`, and `Cargo.toml`.

- [ ] Write failing unit tests for PDF path validation.
- [ ] Add the direct `webview2-com` dependency and Windows-specific command.
- [ ] Register the command and run Rust tests.

### Task 3: Replace print UI with direct export

**Files:** `Toolbar.tsx`, `PdfExportWindow.tsx`, `openPdfExport.ts`, and related CSS.

- [ ] Select the destination using the Save dialog before creating the snapshot.
- [ ] Create the export webview hidden and invoke the native command after readiness.
- [ ] Emit completion, show success/error feedback, and remove `window.print()`.
- [ ] Run frontend tests and build.

### Task 4: Verify and package

- [ ] Run `npm test`, `npm run build`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Build with `npm run tauri build`.
- [ ] Launch the standalone executable for manual direct-export verification.
