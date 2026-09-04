# Foldown 1.0.5

Released 30 August 2026 by Zeeka Limited.

Foldown 1.0.5 is a feature release. It brings direct ChatGPT, Claude, and Gemini integrations alongside Foldown's existing Local Server support, a full retrieval-augmented-generation (RAG) overhaul with optional embeddings and reranking, and a new Markdown cheat sheet. It upgrades an existing Foldown 1.0.4 installation in place.

## Added

### Multi-provider AI chat

- Interactive Mode can now talk directly to **ChatGPT** (OpenAI), **Claude** (Anthropic), or **Gemini** (Google), in addition to any **Local Server** (llama.cpp, Ollama, LM Studio, or any other OpenAI-compatible endpoint).
- A **Provider** selector in AI Settings switches the active chat provider. All four providers' settings (endpoint, API key, chat model) are saved together, so switching providers never discards what you typed into another one.
- The three cloud providers use each provider's own native tool-calling to create, replace, or delete Markdown files; Local Server keeps using its existing text-based JSON contract unchanged. Both paths produce the same validated result.
- Each cloud provider's API key is stored in Windows Credential Manager, protected by your Windows login, never in Foldown's own SQLite settings file.

### Retrieval-augmented generation (RAG) overhaul

- Retrieval now runs a candidate → (optional rerank) → final-selection pipeline instead of a single naive lookup, with configurable candidate count, final count, and character budget.
- **Optional embedding-based retrieval:** point Foldown at any OpenAI-compatible `/embeddings` endpoint (its own endpoint, separate from the chat server) to rank candidates by vector similarity instead of plain full-text search. Configurable Nomic-style document/query embedding prefixes. Embeddings are cached per model and per exact input text, and automatically fall back to full-text search if the server is unreachable or a cached embedding's dimension no longer matches the server's current output.
- **Optional reranker:** point Foldown at any TEI/Jina-compatible `/rerank` endpoint (the same shape llama.cpp's `--reranking` flag serves) to re-score candidates against the exact question before final selection. Automatically falls back to the unreranked order if the reranker is unreachable or misbehaves.
- Chunks now carry a persisted ordinal and per-model embedding dimension tracking, with an automatic one-time schema migration for existing indexes.
- Workspace indexing now runs in the background on workspace open, with **indexing / ready / error** status events shown in the UI.
- Retrieved citations are now clickable and jump straight to the cited excerpt in the source file, tolerant of whitespace/newline differences between the indexed and on-disk text.
- The system prompt now explicitly instructs the model to admit when the retrieved context is insufficient rather than guessing.
- "Scan for local servers" and per-endpoint connection-status badges make it easier to discover and confirm local embedding/reranker servers.

### MD Guide

- A new **MD Guide** button on the editor toolbar opens a Markdown syntax cheat sheet in its own window — headings, bold, italic, strikethrough, blockquotes, lists, task lists, inline code, fenced code blocks, links, images, tables, and horizontal rules, each with its syntax next to its rendered result. The window can stay open alongside the editor, including on a second monitor.

## Changed

- The AI Settings page was redesigned around the provider picker: a **Provider** selector at the top, fields conditional on the selected provider, and Retrieval/Reranking kept as separate sections that apply no matter which chat provider is active.
- Tools > Document conversion buttons now use the same accent color as the Interactive Mode button, and dropped their trailing ellipsis, along with "Open Existing Workspace…" on the welcome screen.
- The README now documents all four chat providers and explains the RAG pipeline, the embedding option, and the reranker option in depth, rather than a brief mention.

## Fixed

- Citation jump-to-text search is now tolerant of whitespace and newline differences between indexed and on-disk text.
- Corrected embedding base URL resolution and added dimension-mismatch detection, so retrieval falls back to full-text search instead of silently scoring every candidate `0.0` when an embedding model's server changes underneath an unchanged model name.
- Windows Credential Manager access is now serialized within the app, eliminating an intermittent write-then-read race that could occur under concurrent load.

## Upgrade behaviour

- `Foldown-1.0.5-Windows-x64-Setup.exe` uses the same application identifier, product name, publisher, per-machine install mode, uninstall registry key, and `C:\Program Files\Foldown` destination as prior releases.
- Running the 1.0.5 installer upgrades and overwrites the existing Foldown installation rather than creating a second application instance.
- User workspaces and Markdown files are not part of the installation directory and are not removed or duplicated by the upgrade.
- Existing AI settings migrate automatically: a pre-1.0.5 flat server/API-key configuration is folded into the new "Local Server" provider block on first read, and any previously-saved API key is moved into Windows Credential Manager.

## Downloads

- `Foldown-1.0.5-Windows-x64-Setup.exe` — per-machine NSIS installer and in-place upgrader.
- `Foldown-1.0.5-Windows-x64-Standalone.exe` — portable application executable.
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

The release was verified with the complete Rust and frontend test suites (144 backend, 49 frontend — up from 93/35 at 1.0.4, reflecting the RAG overhaul and multi-provider work), a production frontend build, and Authenticode signature checks on both binaries. The multi-provider AI Settings UI, provider switching without data loss, and the new MD Guide window were manually verified end-to-end in a running build ahead of this release. A concurrency bug in Windows Credential Manager access (an intermittent write/read race under Rust's parallel test runner) was found and fixed during this release's testing; see Fixed above.

## Licence and support

This release is provided under the [MIT License](LICENSE.md). Contact [support@zeeka.nz](mailto:support@zeeka.nz) for support.
