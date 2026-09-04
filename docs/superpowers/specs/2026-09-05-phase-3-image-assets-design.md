# Phase 3: Image Assets Design

## Goal

Let users drag supported image files into the Markdown editor. Foldown will copy each image into the workspace `assets/` directory, insert a relative Markdown image reference at the cursor, and render the image in Preview.

## Scope

- Accept PNG, JPEG/JPG, GIF, WebP, and SVG files dragged from the operating system onto the editor.
- Create `assets/` when needed and copy the source file into it.
- Preserve the original filename where available; add a numeric suffix on collision rather than overwriting an existing asset.
- Insert `![<filename stem>](assets/<filename>)` at the current editor cursor, followed by a newline when needed.
- Use workspace-relative forward-slash paths in Markdown.
- Render local image references in Preview through a Tauri-safe asset URL.
- Reject unsupported files and failed imports with a user-visible message and no document change.

## Non-goals

- No image resizing, compression, cropping, or editing.
- No remote URL downloading.
- No automatic migration of existing images.
- No deletion or deduplication of unused assets.

## Architecture

The frontend handles the browser drag/drop event and passes the dropped source path plus the active Markdown path to a dedicated Tauri command. Rust validates that the source is an allowed image and copies it into the active workspace's `assets/` directory using existing workspace containment and file-operation helpers. The command returns the workspace-relative asset path; the editor inserts the Markdown reference and existing autosave persists it.

Preview keeps Markdown rendering sanitized, then resolves local image `src` values to Tauri asset URLs only when they are relative workspace paths. External image URLs remain unchanged subject to the existing sanitizer policy.

## Safety and errors

- The active workspace remains the authority for all destination paths.
- Destination paths cannot escape the workspace or `assets/`.
- Source paths must point to regular files with an approved extension; extension matching is case-insensitive.
- Imports never overwrite an existing file.
- A failed copy or invalid source leaves the editor buffer untouched.

## Acceptance criteria

- Dropping `diagram.png` into the editor creates `assets/diagram.png`, inserts a Markdown image reference, autosaves it, and displays it in Preview.
- A second `diagram.png` creates a distinct filename and leaves the first file unchanged.
- Unsupported files are rejected with a clear message.
- Existing Markdown links, external images, editor typing, autosave, and Preview behavior remain intact.
- Frontend tests, Rust tests, frontend build, and a Windows standalone EXE build pass before manual testing.
