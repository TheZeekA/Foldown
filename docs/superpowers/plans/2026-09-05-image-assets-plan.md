# Image Assets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add safe drag-and-drop image importing into `assets/`, Markdown insertion, and local image Preview rendering.

**Architecture:** The editor owns drag/drop and cursor insertion. A Rust command validates and copies the source into the active workspace, returning a relative path. Preview converts local image references to safe Tauri asset URLs after Markdown rendering.

**Tech Stack:** React, TypeScript, CodeMirror, Tauri 2, Rust, `tauri::path::BaseDirectory`, existing workspace authority and file helpers, Vitest, Rust unit tests.

**Spec:** `docs/superpowers/specs/2026-09-05-phase-3-image-assets-design.md`

## Global Constraints

- Supported formats are PNG, JPEG/JPG, GIF, WebP, and SVG.
- Assets are stored only under the workspace-relative `assets/` directory.
- Existing assets are never overwritten; collisions receive numeric suffixes.
- Failed imports must not modify the editor document.
- Existing external URLs and Markdown behavior remain unchanged.

---

### Task 1: Safe Rust asset import

**Files:**
- Modify: `src-tauri/src/commands/files.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/files.rs`

**Interfaces:**
- Produce `import_image_asset(source_path: String, markdown_path: String, workspace_root: String) -> AppResult<String>`.
- Return a workspace-relative path using `/` separators, such as `assets/diagram-1.png`.

- [ ] Write unit tests for accepted extensions, rejected extensions, collision suffixes, and workspace-contained destinations.
- [ ] Run `cargo test commands::files` and verify the new tests fail before implementation.
- [ ] Implement extension validation, `assets/` creation, safe collision naming, and copy using existing `ActiveWorkspace`/filesystem helpers.
- [ ] Register the command in `generate_handler!` and add a typed frontend wrapper.
- [ ] Run `cargo test commands::files` and verify all focused tests pass.
- [ ] Commit with `git add src-tauri/src/commands/files.rs src-tauri/src/lib.rs src/lib/tauriApi.ts && git commit -m "feat: import dropped images into workspace assets"`.

### Task 2: Editor drop handling and Markdown insertion

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/Editor/Editor.tsx`
- Modify: `src/lib/tauriApi.ts`
- Test: `src/components/Editor/imageDrop.test.ts`

**Interfaces:**
- Consume the `importImageAsset` wrapper from Task 1.
- Produce a helper that accepts a dropped file path, active Markdown path, workspace root, and cursor position, then returns the inserted Markdown string or an error.

- [ ] Write failing pure helper tests for a supported image, unsupported file, and generated alt text.
- [ ] Run the focused Vitest test and verify it fails.
- [ ] Implement the helper and editor drop event handling; call the Rust import command before dispatching the Markdown insertion.
- [ ] Keep the existing sidebar file drag/drop behavior unchanged and prevent browser default navigation for image drops over the editor.
- [ ] Display import failures through the existing dialog/message pattern.
- [ ] Run the focused Vitest test and `npm test`.
- [ ] Commit with `git add src/App.tsx src/components/Editor src/lib/tauriApi.ts && git commit -m "feat: insert dropped images into markdown"`.

### Task 3: Preview local asset resolution

**Files:**
- Modify: `src/components/Preview/Preview.tsx`
- Modify: `src/components/Preview/Preview.css` only if required for image sizing
- Test: `src/components/Preview/Preview.test.tsx` or a pure URL-resolution test alongside the component

**Interfaces:**
- Consume the active Markdown path and workspace root from the editor/workspace stores.
- Resolve relative local image references beneath the workspace; leave external URLs unchanged.

- [ ] Write failing tests for `assets/image.png`, a nested Markdown file, an external HTTPS image, and a path escaping the workspace.
- [ ] Run the focused test and verify it fails.
- [ ] Implement safe local URL conversion using Tauri's asset URL mechanism and preserve the existing sanitized Markdown pipeline.
- [ ] Ensure Preview updates when the inserted image changes the editor body.
- [ ] Run the focused Preview test and `npm test`.
- [ ] Commit with `git add src/components/Preview && git commit -m "feat: render local workspace images in preview"`.

### Task 4: Verification and standalone build

**Files:**
- Modify: `README.md` if the existing feature documentation needs the new workflow

- [ ] Run `npm test` and confirm all frontend tests pass.
- [ ] Run `npm run build` and confirm TypeScript and Vite succeed.
- [ ] Run `cargo test` from `src-tauri` and confirm the complete Rust suite passes.
- [ ] Run `git diff --check` and confirm there are no whitespace errors.
- [ ] Build the Windows NSIS bundle so the release executable is produced.
- [ ] Verify the standalone `src-tauri/target/release/foldown.exe` timestamp is from the current build.
- [ ] Push the completed Phase 3 commits to `origin/develop`.
- [ ] Present the standalone EXE for manual testing and wait for approval before Phase 4.
