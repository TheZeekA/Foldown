# Single-File Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Separate Explorer-opened documents from workspaces so the sidebar exposes only the selected Markdown file.

**Architecture:** Add a backend command that activates a file's parent without opening/indexing it as a workspace, then model the frontend session explicitly and render a restricted sidebar in single-file mode.

**Tech Stack:** React, Zustand, TypeScript, Vitest, Tauri v2, Rust.

**Spec:** `docs/superpowers/specs/2026-09-17-single-file-mode-design.md`

## Global Constraints

- Save dirty content before changing sessions.
- Explorer opens always replace the current session with a single-file session.
- Workspace drag-and-drop imports remain unchanged.
- Never expose sibling files in single-file mode.

---

### Task 1: Native single-file activation

- [ ] Write failing Rust tests for valid Markdown, non-Markdown, and missing paths.
- [ ] Implement and register `open_single_file` without recent-workspace or index side effects.
- [ ] Run focused Rust tests.

### Task 2: Frontend session state

- [ ] Write failing store tests for save-before-switch ordering and the synthetic one-file tree.
- [ ] Add the typed API wrapper and `sessionMode`/`openSingleFileAt` store behavior.
- [ ] Run focused frontend tests.

### Task 3: Entry points and sidebar

- [ ] Separate Explorer-open and workspace-drop behavior.
- [ ] Route startup and second-instance events to single-file mode.
- [ ] Restrict the sidebar to one file plus Open Workspace in single-file mode.
- [ ] Run frontend tests and build.

### Task 4: Verify and package

- [ ] Run all frontend and Rust tests.
- [ ] Build and package the standalone executable.
- [ ] Launch the updated executable for manual Explorer-open verification.
