# Phase 2: Knowledge Organization Design

## Goal

Add local-first workspace intelligence for wiki links, backlinks, tags, frontmatter browsing, and actionable health diagnostics without changing the user's Markdown files automatically.

## Scope

### Wiki links and backlinks

- Recognize `[[Project Plan]]` and nested paths such as `[[projects/Project Plan]]`.
- Accept optional display labels such as `[[Project Plan|the plan]]`.
- Resolve targets case-insensitively on Windows.
- Treat targets with or without `.md` as equivalent.
- Show unresolved links clearly.
- Show files that link back to the active document.
- Open targets when a link or backlink is selected.
- Defer heading links such as `[[Project Plan#Next Steps]]`.

### Tags and frontmatter

- Read `tags` as a YAML list or comma-separated string.
- Recognize a singular `tag` field.
- Display tag counts across the workspace.
- Show files associated with a selected tag.
- Open selected files normally.
- Preserve original YAML formatting; this phase does not rewrite frontmatter.
- Report malformed frontmatter through the health checker.

### Workspace health

Report, without automatic modification:

- Broken relative Markdown links.
- Missing image and attachment targets.
- Unresolved wiki links.
- Invalid frontmatter.
- Empty Markdown files.
- Duplicate or ambiguous wiki-link targets.
- Files that could not be indexed or read.

## Non-goals

This phase does not include heading links, automatic link or frontmatter fixes, cloud synchronization, collaboration, or changes to source Markdown formatting.

## Architecture

Extend the existing local SQLite indexing layer rather than creating a second metadata system. Markdown files remain the source of truth; links, tags, and health findings are derived and rebuildable. The React frontend requests workspace metadata through typed Tauri commands and renders it in a Workspace Insights panel with Links, Tags, and Health tabs.

```text
Markdown files on disk
          |
Workspace metadata scan
          |
SQLite-derived metadata
          |
Links / Tags / Health UI
```

The existing filesystem watcher triggers metadata refresh after file creation, modification, rename, and deletion. A complete rebuild must produce the same metadata for the same workspace contents.

## Resolution rules

- Normalize slash direction consistently.
- Resolve relative links from the linking document's folder.
- Treat `Project Plan` and `Project Plan.md` as equivalent.
- Compare paths case-insensitively on Windows.
- Prefer an exact relative-path match.
- Mark basename-only matches ambiguous when multiple files share the name.
- Ignore external URLs, anchors, mail links, and code-block content.
- Report unresolved and ambiguous links without changing source files.

## Derived data

Store the following in the existing SQLite-backed index:

- Normalized document paths and extracted metadata.
- Link records: source path, raw target, resolved target, and resolution status.
- Tags: normalized tag values linked to document paths.
- Health findings: category, severity, path, message, and optional target.

## User experience

Workspace Insights contains three tabs:

- Links: backlinks and unresolved links for the active file.
- Tags: workspace-wide tag counts and matching files.
- Health: grouped findings with severity and affected path.

The views include explicit loading, empty, and error states. Selecting a file opens it through the existing editor flow. Selecting a link or backlink navigates to the relevant file.

## Reliability and privacy

- Unreadable files appear as health findings and do not stop the scan.
- Invalid YAML is reported while the rest of the workspace continues indexing.
- One malformed link cannot prevent other links from being analyzed.
- Stale metadata is replaced during a successful rescan.
- Metadata query errors show in the Insights UI without changing editor state.
- No health action modifies files automatically.
- External links and unsupported link types are ignored rather than reported as broken.
- Workspace metadata is isolated to the active workspace.

## Testing

Tests must cover:

- Wiki-link parsing, display labels, nested paths, missing extensions, URLs, anchors, and code blocks.
- Exact, unresolved, and ambiguous target resolution.
- Backlink queries across nested workspace paths.
- Tag extraction from lists, strings, singular `tag`, empty values, and malformed frontmatter.
- Health findings for broken links, missing assets, invalid frontmatter, empty files, and unreadable files.
- Workspace isolation and case-insensitive Windows path behavior.
- Metadata rebuild after file creation, modification, rename, and deletion.
- Links, Tags, and Health UI, including empty, loading, and error states.
- Existing frontend and Rust test suites remaining green.

## Release acceptance

Phase 2 is ready for user testing only when:

- The complete frontend test suite passes.
- The complete Rust test suite passes.
- The TypeScript/Vite build passes.
- A Windows EXE and installer build successfully.
- The EXE launches and opens a workspace.
- Manual testing confirms links, backlinks, tags, and health findings.
- Phase 3 does not begin until the user approves the Phase 2 EXE.
