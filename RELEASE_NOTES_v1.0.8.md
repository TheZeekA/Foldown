# Foldown 1.0.8

Released 1 September 2026 by Zeeka Limited.

Foldown 1.0.8 is a maintenance release focused on reliable Markdown editing and file management.

## Fixed

- Normal autosaves no longer trigger the “This file changed outside Foldown” warning when Windows delivers the file-watcher notification late.
- Renaming the currently open Markdown file now keeps the editor and file watcher pointed at the new path.
- Renaming a Markdown file without typing `.md` now preserves the `.md` extension.

## Verification

- Frontend tests: 55 passed.
- Rust tests: 146 passed.
- Production frontend build passed.

## Downloads

- `Foldown-1.0.8-Windows-x64-Setup.exe` — Windows installer.
- `Foldown-1.0.8-Windows-x64-Standalone.exe` — portable application executable.

Foldown requires 64-bit Windows and Microsoft Edge WebView2.
