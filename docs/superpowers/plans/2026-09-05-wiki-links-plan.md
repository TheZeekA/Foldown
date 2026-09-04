# Wiki Links and Backlinks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add local `[[wiki links]]`, deterministic target resolution, unresolved-link reporting, and backlinks for the active Markdown file.

**Architecture:** Keep parsing and path resolution pure and testable in Rust. Extend the existing workspace index with derived link records and expose a typed workspace-insights query. React renders links/backlinks and navigates using the existing `openFile` flow.

**Tech Stack:** Rust/Tauri 2, rusqlite, React 19, TypeScript, Zustand, Vitest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-09-05-phase-2-knowledge-organization-design.md`

## Global Constraints

- Recognize `[[Project Plan]]`, nested paths, and optional display labels.
- Treat targets with or without `.md` as equivalent and compare Windows paths case-insensitively.
- Ignore external URLs, anchors, and code-block content.
- Report unresolved and ambiguous links without modifying Markdown files.

### Task 1: Pure wiki-link parsing and resolution

**Files:**
- Create: `src-tauri/src/knowledge/mod.rs`
- Test: `src-tauri/src/knowledge/mod.rs`

**Interfaces:**
- `WikiLink { raw_target: String, display_text: String, target_path: String }`.
- `parse_wiki_links(content: &str) -> Vec<WikiLink>`.
- `resolve_wiki_link(source_relative: &Path, target: &str, documents: &[String]) -> LinkResolution`.
- `LinkResolution = Resolved(String) | Unresolved | Ambiguous(Vec<String>)`.

- [ ] Add tests for display labels, nested paths, optional `.md`, URLs, anchors, fenced code, and duplicate/basename ambiguity.
- [ ] Run `cargo test knowledge` and verify the new tests fail.
- [ ] Implement line-aware fenced-code exclusion, target normalization, relative-folder resolution, exact matching, and ambiguity detection.
- [ ] Run `cargo test knowledge` and verify it passes.
- [ ] Commit with `git add src-tauri/src/knowledge/mod.rs && git commit -m "feat: parse and resolve wiki links"`.

### Task 2: Index link records

**Files:**
- Modify: `src-tauri/src/ai/index.rs`
- Modify: `src-tauri/src/knowledge/mod.rs`
- Test: `src-tauri/src/ai/index.rs`

**Interfaces:**
- `KnowledgeIndex::workspace_links(root: &Path) -> AppResult<Vec<LinkRecord>>`.
- `LinkRecord { source_path: String, raw_target: String, display_text: String, resolved_path: Option<String>, status: LinkStatus }`.

- [ ] Test resolved, unresolved, and ambiguous links across nested files and rebuild behavior after edits.
- [ ] Store or derive link records during workspace sync using the existing indexed document contents and complete document-path list.
- [ ] Ensure stale link records disappear after file deletion or rename.
- [ ] Run the complete Rust suite and commit with `git add src-tauri/src/ai/index.rs src-tauri/src/knowledge/mod.rs && git commit -m "feat: index workspace wiki links"`.

### Task 3: Links Tauri API and UI

**Files:**
- Create: `src/features/Insights/LinksPanel.tsx`
- Create: `src/features/Insights/InsightsPanel.css`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauriApi.ts`
- Modify: `src-tauri/src/commands/knowledge.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/Editor/EditorPane.tsx`
- Test: `src/features/Insights/LinksPanel.test.tsx`

**Interfaces:**
- `getWorkspaceLinks(workspaceRoot: string, activePath: string | null): Promise<WorkspaceLinks>`.
- `WorkspaceLinks = { backlinks: LinkRecord[]; outgoing: LinkRecord[]; unresolved: LinkRecord[] }`.

- [ ] Add wrapper and Rust command tests for active-workspace containment and typed serialization.
- [ ] Implement the query command and Links tab with loading, empty, unresolved, and error states.
- [ ] Use `openFile(record.resolvedPath, workspaceRoot)` when a resolved link/backlink is clicked.
- [ ] Run `npm test`, `npm run build`, and `cargo test`; commit with `git add src/features/Insights src/lib src-tauri/src/commands src-tauri/src/lib.rs src/components/Editor/EditorPane.tsx && git commit -m "feat: add wiki links and backlinks view"`.

### Task 4: Wiki-link verification

- [ ] Test links with spaces, nested folders, missing extensions, duplicate basenames, and links inside code blocks.
- [ ] Modify/create/delete notes and verify insights update after workspace refresh.
- [ ] Confirm no Markdown source file is modified by link analysis.
