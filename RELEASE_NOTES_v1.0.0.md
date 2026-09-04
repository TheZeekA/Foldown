# Foldown 1.0.0

Released 29 August 2026 by Zeeka Limited.

Foldown 1.0.0 is the first official production release of the local-first Markdown workspace for Windows. It combines focused Markdown editing, workspace management, document conversion, and an optional private AI assistant while keeping documents as ordinary files on disk.

## Highlights

- Source, split, and rendered Markdown views powered by CodeMirror 6 and GitHub Flavored Markdown.
- Automatic atomic saves and external-change conflict handling.
- Recent-workspace welcome screen, existing-folder selection, and named workspace creation.
- Full file and folder management, drag-and-drop organisation, Recycle Bin deletion, and workspace search.
- Configurable appearance, editor font, and text size in a dedicated settings interface.
- Conversion of TXT, HTML, CSV, and DOCX documents to Markdown, individually or in batches.
- Optional Interactive Mode for OpenAI-compatible model servers on the local computer or private network.
- Incremental local workspace indexing, full-text retrieval, and optional embedding caching to reduce prompt tokens.
- Automatic validated AI create and replace operations, with explicit confirmation before any AI-requested deletion.
- Runtime model discovery and switching between models offered by the configured endpoint.

## Editing and workspace features

- Markdown syntax highlighting, formatting commands, line wrapping, code folding, tables, task lists, links, fenced code blocks, and strikethrough.
- YAML frontmatter detection with a collapsible field panel.
- Light, dark, and system themes.
- Markdown-only or all-files workspace tree views.
- Create, rename, duplicate, move, and delete files and folders.
- Open Markdown files through Windows or by dropping them onto Foldown.
- Persistent window placement and recent-workspace history without automatically reopening the previous workspace.
- Windows device-path prefixes are normalised in recent-workspace labels for clean, readable paths.

## Interactive Mode

- Streams model responses and supports request cancellation.
- Fetches available models from the configured server and provides an in-chat model selector.
- Retrieves a limited set of relevant Markdown excerpts instead of sending the entire workspace with every request.
- Updates the local index only for new, changed, or deleted files.
- Keeps chat history in the current application session and resets it when the workspace changes.
- Hides machine-readable file actions from chat while applying valid creates and replacements automatically.
- Confirms every requested deletion with an **Are you sure?** prompt and sends confirmed deletions to the Windows Recycle Bin.
- Rejects absolute paths, traversal, non-Markdown targets, symlink escapes, and operations outside the active workspace.
- Revalidates proposed operations immediately before applying them to prevent stale or changed targets from being modified.
- Disables HTTP redirects for AI requests so workspace excerpts cannot be silently redirected to another host.

## Settings and application information

- Separate App Settings, AI Settings, Tools, and About pages.
- AI endpoint, API key, default chat model, and optional embedding model configuration.
- Direct settings link when Interactive Mode is opened without a configured server and model.
- Dynamic software version, Zeeka Limited developer details, support contact, and in-app licence display.
- Installer installs per-machine under Program Files.

## Security and privacy

- The active workspace is canonicalised and enforced as the authority boundary for file, search, settings, and AI commands.
- Filesystem and AI paths are validated at the native command boundary.
- Workspace search and AI indexing skip symbolic links.
- Preview rendering is sanitised and the production webview uses a restrictive Content Security Policy.
- Markdown files, indexes, settings, and optional embedding cache remain local. Only excerpts used for a submitted chat request are sent to the server selected by the user.

## Downloads

- `Foldown-1.0.0-Windows-x64-Setup.exe` — per-machine NSIS installer for Program Files.
- `Foldown-1.0.0-Windows-x64-Standalone.exe` — portable application executable.
- `Zeeka-Limited-Foldown-Self-Signed.cer` — public signing certificate.
- `SHA256SUMS.txt` — SHA-256 checksums for every published binary and certificate.

Foldown requires 64-bit Windows and Microsoft Edge WebView2.

## Signing information

The installer and standalone executable are Authenticode-signed with a self-signed certificate whose subject is `CN=Zeeka Limited` and whose SHA-1 thumbprint is:

```text
356487DAB123E0A290FD454EEB20613497B7E7DF
```

The certificate expires on 29 August 2031. Compare the downloaded certificate's thumbprint with the value above before trusting it. A self-signed certificate provides integrity after it is explicitly trusted, but it does not provide third-party identity validation or Microsoft SmartScreen reputation. Windows may therefore warn until the certificate is installed into an appropriate trusted certificate store.

## Licence and support

This release is provided under the [MIT License](LICENSE.md). Contact [support@zeeka.nz](mailto:support@zeeka.nz) for support.
