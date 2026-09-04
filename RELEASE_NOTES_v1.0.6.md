# Foldown 1.0.6

Released 31 August 2026 by Zeeka Limited.

Foldown 1.0.6 is a maintenance release focused on reliable workspace synchronization and a smoother Interactive Mode experience.

## Fixed

- AI chat no longer rewrites an already-clean open Markdown file before reading it, preventing a false “This file changed outside Foldown” reload prompt.
- The sidebar now watches the active workspace recursively and refreshes when Markdown files are created, removed, or changed outside Foldown. This works even when no file is currently open.

## Verification

- Frontend tests: 50 passed.
- Rust tests relevant to the release: 143 passed. Three Windows Credential Manager tests require an available interactive Windows logon session and remain environment-dependent.
- Production frontend build passed.

## Downloads

- `Foldown-1.0.6-Windows-x64-Setup.exe` — Windows installer.
- `Foldown-1.0.6-Windows-x64-Standalone.exe` — portable application executable.

Foldown requires 64-bit Windows and Microsoft Edge WebView2.
