# Workspace Health and Insights Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Diagnose broken links, missing assets, malformed frontmatter, empty notes, ambiguous links, and unreadable files in one local workspace-health view.

**Architecture:** Add a deterministic health scan over the existing indexed Markdown documents and link/tag metadata. Findings are returned as derived diagnostics grouped by severity and category; the UI is read-only and never auto-fixes files.

**Tech Stack:** Rust/Tauri 2, rusqlite, React 19, TypeScript, Vitest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-09-05-phase-2-knowledge-organization-design.md`

## Global Constraints

- Unreadable files become findings and do not stop the scan.
- Invalid YAML and malformed links do not prevent other files from being analyzed.
- External links and unsupported link types are ignored.
- Health actions never modify files automatically.

### Task 1: Markdown reference scanner

**Files:**
- Modify: `src-tauri/src/knowledge/mod.rs`
- Test: `src-tauri/src/knowledge/mod.rs`

**Interfaces:**
- `scan_markdown_references(source_relative: &Path, content: &str) -> Vec<Reference>`.
- `Reference { target: String, kind: ReferenceKind }`.
- `ReferenceKind = MarkdownLink | Image`.

- [ ] Add tests for relative Markdown links, image paths, query/anchor suffixes, external URLs, mail links, code blocks, and malformed syntax.
- [ ] Run `cargo test knowledge` and verify the new tests fail.
- [ ] Implement a line-aware scanner that strips fragment/query portions for local existence checks and ignores fenced code.
- [ ] Run focused tests and commit with `git add src-tauri/src/knowledge/mod.rs && git commit -m "feat: scan markdown references"`.

### Task 2: Health findings

**Files:**
- Modify: `src-tauri/src/knowledge/mod.rs`
- Modify: `src-tauri/src/ai/index.rs`
- Test: `src-tauri/src/ai/index.rs`

**Interfaces:**
- `KnowledgeIndex::workspace_health(root: &Path) -> AppResult<Vec<HealthFinding>>`.
- `HealthFinding { category: String, severity: String, path: String, message: String, target: Option<String> }`.

- [ ] Test broken Markdown links, missing images, unresolved/ambiguous wiki links, invalid frontmatter, empty files, and unreadable files.
- [ ] Combine reference scan, link resolution, and tag/frontmatter errors into stable findings sorted by path/category/message.
- [ ] Use `warning` for empty notes, unresolved links, and missing assets; use `error` for invalid frontmatter and unreadable files.
- [ ] Ensure all workspace paths are relative and no finding leaks content outside the active workspace.
- [ ] Run the complete Rust suite and commit with `git add src-tauri/src/knowledge src-tauri/src/ai/index.rs && git commit -m "feat: diagnose workspace health"`.

### Task 3: Insights container and health UI

**Files:**
- Create: `src/features/Insights/InsightsPanel.tsx`
- Create: `src/features/Insights/InsightsPanel.css`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauriApi.ts`
- Modify: `src-tauri/src/commands/knowledge.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/Sidebar/Sidebar.tsx`
- Test: `src/features/Insights/InsightsPanel.test.tsx`

**Interfaces:**
- `getWorkspaceHealth(workspaceRoot: string): Promise<HealthFinding[]>`.
- `WorkspaceInsights` owns the active tab: `"links" | "tags" | "health"`.

- [ ] Add typed command tests and UI tests for tab switching, loading, empty, error, severity grouping, and file navigation.
- [ ] Add a Sidebar Insights button that opens the panel without replacing the current editor or workspace tree state.
- [ ] Render Health findings grouped by category with severity labels and affected paths.
- [ ] Render the Links and Tags panels from the shared APIs created by their plans.
- [ ] Refresh insights after workspace/file watcher refreshes and after opening a different document.
- [ ] Run `npm test`, `npm run build`, and `cargo test`; commit with `git add src/features/Insights src/components/Sidebar/Sidebar.tsx src/lib src-tauri/src/commands src-tauri/src/lib.rs && git commit -m "feat: add workspace insights panel"`.

### Task 4: Phase 2 verification

- [ ] Create a fixture workspace containing valid links, broken links, wiki links, duplicate basenames, tags, malformed frontmatter, empty notes, and missing images.
- [ ] Verify Links, Tags, and Health tabs show deterministic results and clicking files uses the normal editor flow.
- [ ] Modify, rename, and delete files and verify stale findings disappear after refresh.
- [ ] Confirm no diagnostic operation changes any workspace file.
