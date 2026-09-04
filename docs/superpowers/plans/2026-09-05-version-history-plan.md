# Local Version History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide bounded, workspace-isolated local snapshots with diff preview and safe restore.

**Architecture:** Add a dedicated SQLite-backed `HistoryStore` using the existing application-data database path. Tauri commands enforce workspace containment and external-change checks. The editor records the last saved content and requests best-effort snapshots before saves; a history panel handles listing, diffing, confirmation, deletion, and restore.

**Tech Stack:** React 19, TypeScript, Zustand, Tauri 2, Rust, rusqlite, Vitest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-09-05-phase-1-editor-safety-and-ai-design.md`

## Global Constraints

- Snapshots are stored outside user workspaces in local application data.
- The default retention limit is 50 snapshots per file and identical contents are not duplicated.
- A history failure must not prevent a normal save.
- Restore requires preview and confirmation, preserves the current content first, and aborts on external changes.
- Workspace history is isolated by canonical workspace path and relative file path.

### Task 1: History storage model

**Files:**
- Create: `src-tauri/src/history/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/history/mod.rs`

**Interfaces:**
- `HistoryEntry { id: i64, workspace_root: String, relative_path: String, created_at: i64, byte_length: i64 }`.
- `HistoryStore::open(db_path: PathBuf) -> AppResult<Self>`.
- `record_snapshot(workspace_root: &Path, relative_path: &Path, content: &str, retention: usize) -> AppResult<()>`.
- `list_snapshots(workspace_root: &Path, relative_path: &Path) -> AppResult<Vec<HistoryEntry>>`.
- `snapshot_content(id: i64) -> AppResult<String>`.
- `delete_snapshot(id: i64) -> AppResult<()>`.
- `clear_snapshots(workspace_root: &Path, relative_path: Option<&Path>) -> AppResult<()>`.

- [ ] Write Rust tests for schema creation, duplicate suppression, newest-first ordering, retention pruning, deletion, clearing, and workspace/path isolation.
- [ ] Run `cargo test history` from `src-tauri` and verify the tests fail.
- [ ] Implement the schema and methods with parameterized SQL, canonical workspace keys, and bounded pruning.
- [ ] Run `cargo test history` and verify all tests pass.
- [ ] Commit with `git add src-tauri/src/history/mod.rs src-tauri/src/lib.rs && git commit -m "feat: add local history storage"`.

### Task 2: History commands and save integration

**Files:**
- Create: `src-tauri/src/commands/history.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/commands/files.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauriApi.ts`
- Test: `src-tauri/src/commands/history.rs`
- Test: `src/lib/tauriApi.test.ts`

**Interfaces:**
- `list_history(workspace_root: String, path: String) -> AppResult<Vec<HistoryEntry>>`.
- `get_history_content(id: i64) -> AppResult<String>`.
- `delete_history_snapshot(id: i64) -> AppResult<()>`.
- `clear_history(workspace_root: String, path: Option<String>) -> AppResult<()>`.
- `restore_history_snapshot(id: i64, workspace_root: String, path: String) -> AppResult<()>`.
- `record_history_snapshot(workspace_root: String, path: String, content: String) -> AppResult<()>`.

- [ ] Test command path containment, external-content mismatch on restore, current-version preservation, and best-effort snapshot failure behavior.
- [ ] Add typed frontend wrappers and invoke contract tests.
- [ ] Register `HistoryStore` with the app using the same app-data database path and register all commands.
- [ ] Make restore verify the active workspace, read current disk content, snapshot it, and write the selected content atomically before refreshing the search index.
- [ ] Keep ordinary `save_file` unchanged; the frontend will call `record_history_snapshot` best-effort before it calls `save_file`, ensuring a history failure cannot block saving.
- [ ] Run Rust and frontend focused tests, then complete suites.
- [ ] Commit with `git add src-tauri/src/commands src-tauri/src/lib.rs src/lib/tauriApi.ts src/lib/tauriApi.test.ts && git commit -m "feat: expose safe history commands"`.

### Task 3: Editor save bookkeeping

**Files:**
- Modify: `src/stores/editor.ts`
- Test: `src/stores/editor.test.ts`

**Interfaces:**
- Add `lastSavedContent: string` to `EditorState`.
- On open/reload, set it to disk content.
- In `saveNow`, call `recordHistorySnapshot(workspaceRoot, openPath, lastSavedContent)` only when dirty and content differs; swallow only that call's error, then call existing `saveFile`.

- [ ] Test snapshot requests contain the previous content, duplicate saves do not snapshot identical content, and snapshot failures still call `saveFile`.
- [ ] Implement bookkeeping and reset it after a successful save, reload, and workspace reset.
- [ ] Run `npm test -- src/stores/editor.test.ts` and verify it passes.
- [ ] Commit with `git add src/stores/editor.ts src/stores/editor.test.ts && git commit -m "feat: snapshot previous content before saves"`.

### Task 4: History UI and restore flow

**Files:**
- Create: `src/features/History/HistoryPanel.tsx`
- Create: `src/features/History/HistoryPanel.css`
- Create: `src/features/History/historyDiff.ts`
- Create: `src/features/History/historyDiff.test.ts`
- Modify: `src/components/Editor/Toolbar.tsx`
- Modify: `src/components/Editor/EditorPane.tsx`
- Test: `src/features/History/HistoryPanel.test.tsx`

**Interfaces:**
- `buildHistoryDiff(current: string, previous: string): DiffLine[]` where `DiffLine` is `{ kind: "same" | "added" | "removed"; text: string }`.
- Panel receives `workspaceRoot` and `path`, and uses typed history wrappers.

- [ ] Test diff rendering, newest-first entries, loading/error/empty states, delete confirmation, clear confirmation, and restore confirmation.
- [ ] Implement a panel showing timestamp, size, diff preview, restore, delete, and clear actions.
- [ ] On restore, re-read the active file state and call `restoreHistorySnapshot`; reload the editor only after success.
- [ ] Add accessible labels, keyboard focus behavior, and styles consistent with existing settings/proposal panels.
- [ ] Run `npm test`, `npm run build`, and `cd src-tauri; cargo test`.
- [ ] Commit with `git add src/features/History src/components/Editor/Toolbar.tsx src/components/Editor/EditorPane.tsx && git commit -m "feat: add local version history panel"`.

### Task 5: Version history verification

- [ ] Edit and save a file several times, verify snapshots are created and bounded.
- [ ] Restore an old snapshot and verify the current content is preserved as a newer snapshot.
- [ ] Modify the file externally before restore and verify Foldown refuses to overwrite it.
- [ ] Test two workspaces with identical relative filenames and verify their histories never mix.
