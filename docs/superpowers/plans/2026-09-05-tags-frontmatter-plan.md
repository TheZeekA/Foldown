# Tags and Frontmatter Browser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a workspace-wide tag browser from existing YAML frontmatter without rewriting user files.

**Architecture:** Reuse `splitFrontmatter` semantics in a Rust metadata extractor so workspace scans remain authoritative. Normalize `tags` lists, comma-separated strings, and singular `tag` values into derived tag records, then expose counts and matching files through the Insights API.

**Tech Stack:** Rust/Tauri 2, serde_yaml, rusqlite, React 19, TypeScript, Vitest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-09-05-phase-2-knowledge-organization-design.md`

## Global Constraints

- Support YAML list and comma-separated string values for `tags`.
- Recognize singular `tag`.
- Preserve original YAML formatting and never rewrite files.
- Invalid YAML is reported through health findings while other files continue scanning.

### Task 1: Tag extraction model

**Files:**
- Modify: `src-tauri/src/knowledge/mod.rs`
- Test: `src-tauri/src/knowledge/mod.rs`

**Interfaces:**
- `extract_tags(frontmatter: &str) -> TagExtraction`.
- `TagExtraction { tags: Vec<String>, error: Option<String> }`.
- `normalize_tag(value: &str) -> Option<String>`.

- [ ] Add tests for list tags, comma-separated strings, singular `tag`, whitespace/case normalization, empty values, and malformed YAML.
- [ ] Run `cargo test knowledge` and verify new tests fail.
- [ ] Implement extraction using the same frontmatter boundary rules as the frontend, preserving errors rather than discarding files.
- [ ] Run focused tests and commit with `git add src-tauri/src/knowledge/mod.rs && git commit -m "feat: extract tags from frontmatter"`.

### Task 2: Metadata tag index and queries

**Files:**
- Modify: `src-tauri/src/ai/index.rs`
- Modify: `src-tauri/src/knowledge/mod.rs`
- Test: `src-tauri/src/ai/index.rs`

**Interfaces:**
- `KnowledgeIndex::workspace_tags(root: &Path) -> AppResult<Vec<TagSummary>>`.
- `KnowledgeIndex::files_for_tag(root: &Path, tag: &str) -> AppResult<Vec<String>>`.
- `TagSummary { tag: String, count: usize }`.

- [ ] Test counts, matching files, case normalization, nested files, empty tags, and stale metadata removal after edits.
- [ ] Derive tags from indexed Markdown content during sync and return stable alphabetical summaries.
- [ ] Preserve frontmatter parse errors for the health subsystem rather than making tag queries fail globally.
- [ ] Run `cargo test` and commit with `git add src-tauri/src/ai/index.rs src-tauri/src/knowledge/mod.rs && git commit -m "feat: index workspace tags"`.

### Task 3: Tags UI and API

**Files:**
- Modify: `src/features/Insights/InsightsPanel.tsx`
- Modify: `src/features/Insights/InsightsPanel.css`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauriApi.ts`
- Modify: `src-tauri/src/commands/knowledge.rs`
- Test: `src/features/Insights/InsightsPanel.test.tsx`

**Interfaces:**
- `getWorkspaceTags(workspaceRoot: string): Promise<TagSummary[]>`.
- `getFilesForTag(workspaceRoot: string, tag: string): Promise<string[]>`.

- [ ] Add typed invoke tests and UI tests for empty/loading/error states and tag selection.
- [ ] Render alphabetical tag counts; selecting a tag shows matching files and opens a selected file through `openFile`.
- [ ] Ensure tag values are displayed in normalized form while source files remain unchanged.
- [ ] Run `npm test`, `npm run build`, and `cargo test`; commit with `git add src/features/Insights src/lib src-tauri/src/commands && git commit -m "feat: add workspace tag browser"`.

### Task 4: Tags verification

- [ ] Test list, string, singular, empty, and malformed frontmatter in real workspace files.
- [ ] Edit frontmatter and verify counts refresh without changing formatting.
- [ ] Confirm selecting a tag opens the expected Markdown file.
