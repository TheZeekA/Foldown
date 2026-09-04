# Selection-based AI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users run focused AI transformations on selected Markdown text and accept the result through a safe diff.

**Architecture:** Add a dedicated Tauri command that reuses provider dispatch and local retrieval but cannot create, replace, or delete files. The frontend captures a CodeMirror range, requests a proposed replacement, and renders a proposal card that applies only that range after explicit confirmation.

**Tech Stack:** React 19, TypeScript, Zustand, CodeMirror 6, Rust/Tauri 2, reqwest provider clients, Vitest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-09-05-phase-1-editor-safety-and-ai-design.md`

## Global Constraints

- No document content changes until the user accepts a proposal.
- The request contains selected text, action, and relevant workspace context.
- AI failure, cancellation, invalid output, or empty output leaves the document unchanged.
- Existing Interactive Mode and file-action behavior must remain unchanged.

### Task 1: Selection action contracts

**Files:**
- Create: `src/features/SelectionAi/selectionActions.ts`
- Test: `src/features/SelectionAi/selectionActions.test.ts`
- Modify: `src/lib/types.ts`

**Interfaces:**
- `SelectionAiAction = "explain" | "summarize" | "rewrite" | "clarify" | "checklist" | "action-items" | "translate"`.
- `buildSelectionPrompt(action: SelectionAiAction, selectedText: string): string`.
- `SelectionAiResult = { text: string; citations: AiContextChunk[] }`.

- [ ] Write tests for each action label, whitespace trimming, and rejection of empty selection.
- [ ] Run the focused tests and verify they fail.
- [ ] Implement stable prompts that request only a replacement/answer for the selection and preserve Markdown where applicable.
- [ ] Run the focused tests and verify they pass.
- [ ] Commit with `git add src/features/SelectionAi/selectionActions.ts src/features/SelectionAi/selectionActions.test.ts src/lib/types.ts && git commit -m "feat: define selection ai actions"`.

### Task 2: Backend selection command

**Files:**
- Modify: `src-tauri/src/ai/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauriApi.ts`
- Test: `src-tauri/src/ai/commands.rs`
- Test: `src/lib/tauriApi.test.ts`

**Interfaces:**
- Rust `SelectionAiResult { text: String, citations: Vec<ContextChunk> }`.
- Tauri command `run_selection_ai(window, store, index, runtime, active, workspace_root, request_id, action, selected_text, active_path) -> AppResult<SelectionAiResult>`.
- TypeScript `runSelectionAi(workspaceRoot: string, requestId: string, action: SelectionAiAction, selectedText: string, activePath: string): Promise<SelectionAiResult>`.

- [ ] Add backend tests proving the selection prompt cannot produce file actions and empty selection is rejected.
- [ ] Add wrapper tests asserting the exact invoke command and camelCase argument names.
- [ ] Run `cargo test -p foldown` from `src-tauri` and `npm test -- src/lib/tauriApi.test.ts`; verify new tests fail.
- [ ] Refactor shared provider dispatch only as needed, add retrieval using the selected text as query, and return plain proposed text plus citations without proposal creation or auto-application.
- [ ] Register the command in `src-tauri/src/lib.rs` and implement the typed frontend wrapper.
- [ ] Run focused Rust and frontend tests, then the complete suites.
- [ ] Commit with `git add src-tauri/src/ai/commands.rs src-tauri/src/lib.rs src/lib/tauriApi.ts src/lib/tauriApi.test.ts && git commit -m "feat: add safe selection ai command"`.

### Task 3: Editor selection bridge

**Files:**
- Modify: `src/stores/editor.ts`
- Modify: `src/components/Editor/Editor.tsx`
- Test: `src/stores/editor.test.ts`

**Interfaces:**
- Add `selectionRange: { from: number; to: number } | null`.
- Add `replaceRange(from: number, to: number, replacement: string): void`.

- [ ] Test selection state updates and replacement dispatch behavior with a mocked `EditorView`.
- [ ] Run focused tests and verify they fail.
- [ ] Report CodeMirror selection changes from the editor update listener and implement `replaceRange` through a user-event transaction so normal dirty/autosave behavior applies.
- [ ] Run focused tests and verify they pass.
- [ ] Commit with `git add src/stores/editor.ts src/components/Editor/Editor.tsx src/stores/editor.test.ts && git commit -m "feat: expose editor selection actions"`.

### Task 4: Proposal UI and integration

**Files:**
- Create: `src/features/SelectionAi/SelectionAiToolbar.tsx`
- Create: `src/features/SelectionAi/SelectionAiToolbar.css`
- Create: `src/features/SelectionAi/SelectionAiProposal.tsx`
- Create: `src/features/SelectionAi/SelectionAiProposal.css`
- Modify: `src/components/Editor/Toolbar.tsx`
- Modify: `src/components/Editor/EditorPane.tsx`
- Test: `src/features/SelectionAi/SelectionAiProposal.test.tsx`

**Interfaces:**
- Proposal state is local to the editor pane: `{ action, from, to, selectedText, result, citations } | null`.

- [ ] Test accept, reject, retry, copy, loading, cancellation, and error states.
- [ ] Implement action controls disabled without a non-empty selection and a diff showing old/new text.
- [ ] On accept, verify the selection still matches the captured text; otherwise show an external/editor-change warning and do not replace.
- [ ] Apply the range through `replaceRange`, then allow existing autosave to persist it.
- [ ] Run `npm test` and `npm run build`; verify existing Interactive Mode tests remain green.
- [ ] Commit with `git add src/features/SelectionAi src/components/Editor/Toolbar.tsx src/components/Editor/EditorPane.tsx && git commit -m "feat: add selection ai proposal workflow"`.

### Task 5: Selection AI verification

- [ ] Test with each configured provider and with an unavailable provider.
- [ ] Verify no request changes the file before acceptance and accepted changes affect only the original range.
- [ ] Verify cancellation and a changed selection cannot overwrite newer text.
