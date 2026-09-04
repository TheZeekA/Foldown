# Document Outline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a live, nested Markdown heading outline that navigates the active CodeMirror editor.

**Architecture:** Keep heading parsing pure in `src/editor/outline.ts`. `EditorPane` renders the outline from the editor store's body, while `Editor` reports and consumes document-position jump requests through Zustand. No Tauri or file-format changes are required.

**Tech Stack:** React 19, TypeScript, Zustand, CodeMirror 6, Vitest, CSS.

**Spec:** `docs/superpowers/specs/2026-09-05-phase-1-editor-safety-and-ai-design.md`

## Global Constraints

- The outline refreshes without saving the document.
- Documents without headings show an empty state.
- Existing source, split, and preview modes must continue to work.
- Existing frontend tests must remain green.

### Task 1: Heading parser

**Files:**
- Create: `src/editor/outline.ts`
- Test: `src/editor/outline.test.ts`

**Interfaces:**
- Produces `extractMarkdownHeadings(markdown: string): MarkdownHeading[]`.
- `MarkdownHeading` is `{ text: string; level: number; from: number; line: number }`.

- [ ] Write tests for ATX headings from levels 1–6, nested ordering, duplicate titles, headings with trailing `#` markers, indented code blocks, and empty input.
- [ ] Run `npm test -- src/editor/outline.test.ts` and verify the new tests fail because the parser does not exist.
- [ ] Implement a line-based parser that tracks absolute UTF-16 offsets, ignores fenced/indented code, trims heading text, and returns source order.
- [ ] Run `npm test -- src/editor/outline.test.ts` and verify all parser tests pass.
- [ ] Commit with `git add src/editor/outline.ts src/editor/outline.test.ts && git commit -m "feat: parse markdown document headings"`.

### Task 2: Editor jump state

**Files:**
- Modify: `src/stores/editor.ts`
- Modify: `src/components/Editor/Editor.tsx`
- Test: `src/stores/editor.test.ts`

**Interfaces:**
- Add `pendingJumpPosition: number | null` to `EditorState`.
- Add `jumpToPosition(position: number): void` and `clearPendingJumpPosition(): void`.

- [ ] Add store tests proving a requested position is stored and cleared.
- [ ] Run the focused store tests and verify the new assertions fail.
- [ ] Implement the state actions and reset the position on file/workspace reset.
- [ ] Update `Editor.tsx` to dispatch a CodeMirror selection and scroll when `pendingJumpPosition` changes, then clear it.
- [ ] Run `npm test -- src/stores/editor.test.ts` and verify it passes.
- [ ] Commit with `git add src/stores/editor.ts src/components/Editor/Editor.tsx src/stores/editor.test.ts && git commit -m "feat: navigate editor by document position"`.

### Task 3: Outline UI

**Files:**
- Create: `src/components/Editor/DocumentOutline.tsx`
- Create: `src/components/Editor/DocumentOutline.css`
- Modify: `src/components/Editor/EditorPane.tsx`
- Modify: `src/components/Editor/EditorPane.css`
- Test: `src/components/Editor/DocumentOutline.test.tsx`

**Interfaces:**
- `DocumentOutline` consumes `body: string` and calls `jumpToPosition(heading.from)` from the editor store.

- [ ] Write component tests for nested headings, click navigation, and the no-headings empty state.
- [ ] Run the focused component tests and verify they fail.
- [ ] Implement an accessible outline with buttons, heading indentation, stable keys based on offset, and a concise empty state.
- [ ] Place the outline beside the editor in source and split modes without hiding the existing preview.
- [ ] Add theme-compatible styles, keyboard focus states, and a narrow-width layout.
- [ ] Run `npm test` and `npm run build` and verify all tests and the frontend build pass.
- [ ] Commit with `git add src/components/Editor/DocumentOutline.tsx src/components/Editor/DocumentOutline.css src/components/Editor/EditorPane.tsx src/components/Editor/EditorPane.css src/components/Editor/DocumentOutline.test.tsx && git commit -m "feat: add live document outline"`.

### Task 4: Outline verification

- [ ] Open representative short and long Markdown files in `npm run tauri dev`.
- [ ] Verify headings update while typing, clicking navigates correctly, and preview/source modes remain usable.
- [ ] Record any manual issue before starting the next subsystem.
