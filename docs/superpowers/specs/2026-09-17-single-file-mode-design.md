# Single-File Mode Design

## Goal

Opening a Markdown file through Windows Explorer must show only that file in
Foldown's sidebar and editor. Explicitly opening a workspace retains the full
workspace tree and features.

## Session model

The workspace store tracks `sessionMode` as `"none"`, `"workspace"`, or
`"single-file"`. A single-file session retains the selected file's parent as
an internal filesystem authority root, but exposes a synthetic one-file tree.
It does not add the parent to recent workspaces or start workspace indexing.

## Entry points

- Startup file arguments and second-instance `open-file-request` events switch
  to single-file mode, replacing any current workspace after dirty edits save.
- Opening another workspace switches back to workspace mode.
- Dragging an external file onto an existing workspace preserves the current
  import behavior. This uses a separate function from Explorer file opening.

## Backend

Add `open_single_file(path)` which canonicalizes an existing `.md` file,
activates its parent as the internal authority root, and returns canonical
`path` and `root`. It does not touch recent workspaces or the search/AI index.

## UI

Single-file mode shows a sidebar header labelled `Single File`, an **Open
Workspace** action, and exactly one file row. Workspace-only search, insights,
AI, show-all, create, rename, duplicate, delete, and workspace switching menus
are hidden or disabled. The editor and history continue to work normally.

## Error and safety behavior

The current editor is reset only after its dirty content saves successfully.
Invalid, missing, non-Markdown, or parentless paths are rejected. A failed
switch leaves the prior session available and records the error.

## Verification

Tests cover backend validation, frontend transition ordering and synthetic tree
construction, Explorer-open replacement behavior, and preservation of external
drag import behavior. Run frontend tests/build, Rust tests, and package the app.
