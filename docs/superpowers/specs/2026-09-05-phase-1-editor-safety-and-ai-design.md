# Phase 1: Editor Safety and AI Design

## Goal

Improve Foldown's daily editing workflow with document navigation, safe selection-based AI assistance, and local version history. The phase must preserve Foldown's local-first storage model and ensure that AI changes and history restores are reviewable and reversible.

## Scope

### Document outline

- Parse Markdown headings from the active document.
- Display headings in a collapsible, nested outline panel.
- Navigate to a heading when it is selected.
- Refresh the outline as the document changes without saving the document.
- Provide a useful empty state for documents without headings.

### Selection-based AI tools

Provide actions for selected text:

- Explain
- Summarize
- Rewrite
- Improve clarity
- Convert to checklist
- Extract action items
- Translate

Actions use the existing AI provider and retrieval infrastructure. The request includes the selected text, the requested operation, and relevant workspace context. Results appear as proposed replacements with a diff. The user may accept, reject, retry, or copy the result. No document content changes until the user accepts the proposal.

### Local version history

- Capture the previous content before a successful save when the content differs from the newest snapshot; do not create duplicate snapshots for identical content.
- Store snapshots in Foldown's local application data, not inside the workspace.
- Record workspace identity, relative path, timestamp, and content.
- List snapshots newest-first for the active file.
- Show a diff between a snapshot and the current document.
- Require confirmation before restoring a snapshot.
- Preserve the current version before applying a restore.
- Permit individual history deletion and clearing history.
- Bound history by a configurable retention limit, defaulting to 50 snapshots per file.

## Non-goals

This phase does not include persistent AI conversations, workspace-wide history browsing, automatic AI changes without confirmation, wiki links, tags, attachments, health checks, or export formats.

## Architecture

The features share an editor action flow:

```text
Selection or document change
          ↓
Editor action
          ↓
Preview / diff / confirmation
          ↓
Apply safely to disk
          ↓
Create history snapshot
```

The React frontend owns interaction state, selection state, outline display, and proposal/confirmation UI. Existing Tauri commands and filesystem safeguards remain the boundary for atomic writes, workspace containment, and external-change checks. History persistence belongs in Foldown's local application data and must be isolated by canonical workspace path and relative file path.

AI requests continue through the configured provider. Local history is never sent to an AI provider unless the user explicitly selects historical content for a later AI operation.

## Data flow

### AI replacement

1. User selects text and invokes an AI action.
2. Foldown builds a request from the selection, action, and relevant workspace context.
3. The configured provider streams or returns the proposed result.
4. Foldown displays the result in a diff/proposal panel.
5. Accepting replaces only the original selection.
6. The previous file content is snapshotted before an accepted edit is saved atomically.

### History restore

1. User selects a snapshot from the active file's history.
2. Foldown displays a diff against the current content.
3. The user confirms the restore.
4. Foldown verifies that the file has not changed externally.
5. The current version is snapshotted.
6. The selected version is written atomically.

## Reliability and privacy

- Failed history writes must not prevent a normal file save.
- AI failures, cancellation, invalid responses, and empty responses leave the document unchanged.
- External changes invalidate pending AI replacements and restores.
- Corrupt history entries are skipped with a recoverable warning.
- Clearing history requires confirmation.
- Workspace history is isolated so one workspace cannot access another workspace's snapshots.

## Testing

Tests must cover:

- Heading extraction, nesting, duplicate headings, and empty documents.
- Outline navigation to correct editor positions.
- Selection-based AI request construction.
- AI cancellation and provider failure behavior.
- Diff generation and replacement application.
- Atomic save after accepted AI changes.
- Snapshot creation, retention, deletion, and restore.
- Protection against restoring over externally modified files.
- Workspace isolation for history.
- Existing frontend and Rust test suites remaining green.

## Release acceptance

Phase 1 is ready for user testing only when:

- Frontend tests pass.
- Rust tests pass.
- TypeScript and Vite builds pass.
- A Windows release build succeeds.
- The generated EXE launches successfully.
- The EXE is handed to the user for manual testing.

Phase 2 must not begin until the user confirms Phase 1 is acceptable or provides feedback.
