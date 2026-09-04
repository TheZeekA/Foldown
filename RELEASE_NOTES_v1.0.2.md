# Foldown 1.0.2

Released 29 August 2026 by Zeeka Limited.

Foldown 1.0.2 is a maintenance release focused on data safety: it closes a gap where Interactive Mode could overwrite an existing file with no review step, fixes a case where switching workspaces could silently discard unsaved edits, and corrects a set of smaller filesystem, document-conversion, and editor issues found during a full review of the codebase. It upgrades an existing Foldown 1.0.1 installation in place.

## Fixed

### Interactive Mode

- Replacing or deleting an existing file now always shows a confirmation card — with a diff for replacements — before anything on disk changes. Previously, a replace action could apply immediately to any file in the workspace with no review step; only creating a brand-new file still applies automatically, since that can't destroy existing content.
- The confirmation card now correctly labels itself **Create**, **Replace**, or **Delete** to match the action it's confirming, instead of always saying **Delete**.
- If a multi-file Interactive Mode request partially succeeds and then fails, Foldown now reports which file(s) it couldn't create instead of surfacing an opaque error, and refreshes the workspace tree so it never goes stale relative to disk.
- Interactive Mode's instructions to the model now make clear that describing a file change in prose is not the same as performing it, and that Foldown's own interface already asks for confirmation before replacing or deleting a file — so the model shouldn't ask permission itself. Verified against a real local model: replies changed from hedged, sometimes inactive prose to direct statements that reliably included the actual action.
- The newest reply in Interactive Mode is now reliably scrolled into view. The auto-scroll previously targeted an element outside the actual scrolling message list, so the latest reply could stay out of view until you scrolled down manually.

### Workspace and editor safety

- Switching workspaces (or creating a new one) while a file has unsaved edits no longer silently discards them. Foldown now saves the current file before switching, and cancels the switch — leaving your edit in place — if that save fails, instead of losing it with no way back.
- Clicking one file and then quickly clicking another, or switching workspaces, before the first file finishes loading no longer risks its stale content overwriting the file you actually selected, or the file watcher ending up out of sync with what's on screen.

### Filesystem

- Renaming a file to a version of its own name that only differs by letter case (for example `readme.md` to `README.md`) now works, instead of always failing with "already exists" on Windows.
- Sidebar and file-tree paths no longer leak Windows' internal `\\?\` extended-length path prefix, which previously broke integration with Windows' Open Recent and Jump List for files opened from the sidebar.
- Two saves to the same file happening at nearly the same time — for example autosave firing while a manual save is in flight — can no longer corrupt each other; each save now uses its own temporary file.
- A symlink inside the workspace is now rejected instead of silently followed, closing a path where a file operation could act on a location outside the intended target.
- Opening a Markdown file that starts with a byte-order mark no longer leaves a stray invisible character that could stop a leading heading from rendering correctly. A genuinely non-UTF-8 file now fails with a clear message instead of an opaque I/O error.

### Document conversion

- Converting Word documents and CSV files to Markdown now escapes characters such as `*`, `_`, `` ` ``, `[`, and `]` found in the source text, so they no longer get misread as Markdown formatting in the converted output.
- A multi-line cell in a converted CSV or Word table no longer breaks the generated Markdown table's structure.

### Editor and search

- Dragging a folder in the sidebar tree is now correctly blocked from being dropped into its own subfolder even when the two paths use different slash styles; the same fix also corrects how an externally opened file is matched against the current workspace.
- Clicking a multi-word search result now jumps to the matching phrase itself, rather than only the first word of the query.
- The heading-level toolbar button now cycles through all six heading levels instead of jumping straight to plain text after Heading 3.
- The font-size field in App Settings no longer silently reverts input you're in the middle of typing.

## Upgrade behaviour

- `Foldown-1.0.2-Windows-x64-Setup.exe` uses the same application identifier, product name, publisher, per-machine install mode, uninstall registry key, and `C:\Program Files\Foldown` destination as prior releases.
- Running the 1.0.2 installer upgrades and overwrites the existing Foldown installation rather than creating a second application instance.
- User workspaces and Markdown files are not part of the installation directory and are not removed or duplicated by the upgrade.

## Downloads

- `Foldown-1.0.2-Windows-x64-Setup.exe` — per-machine NSIS installer and in-place upgrader.
- `Foldown-1.0.2-Windows-x64-Standalone.exe` — portable application executable.
- `Zeeka-Limited-Foldown-Self-Signed.cer` — public signing certificate.
- `SHA256SUMS.txt` — SHA-256 checksums for every published binary and certificate.

Foldown requires 64-bit Windows and Microsoft Edge WebView2.

## Signing information

The installer and standalone executable are Authenticode-signed with a self-signed certificate whose subject is `CN=Zeeka Limited` and whose SHA-1 thumbprint is:

```text
356487DAB123E0A290FD454EEB20613497B7E7DF
```

The certificate expires on 29 August 2031. Compare the downloaded certificate's thumbprint with the value above before trusting it. A self-signed certificate provides integrity after it is explicitly trusted, but it does not provide third-party identity validation or Microsoft SmartScreen reputation.

## Verification

The release was verified with the complete Rust and frontend test suites (89 backend, 35 frontend), a production frontend build, Authenticode signature checks on both binaries, SHA-256 checksums, and a full interactive pass through Interactive Mode's create/replace/delete flow — including the confirmation gating and auto-scroll fixes — driven end-to-end against a real local model server via Tauri's WebDriver automation.

## Licence and support

This release is provided under the [MIT License](LICENSE.md). Contact [support@zeeka.nz](mailto:support@zeeka.nz) for support.
