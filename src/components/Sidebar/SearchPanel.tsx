import { useEffect, useRef } from "react";
import "./SearchPanel.css";
import { useSearchStore } from "../../stores/search";
import { useEditorStore } from "../../stores/editor";
import { useWorkspaceStore } from "../../stores/workspace";
import type { SearchResult } from "../../lib/types";

/** Matches MARK_START/MARK_END from search/index.rs — control characters
 * (not visible punctuation) so a split can never collide with real note text.
 * Built from char codes rather than a literal regex so no raw control byte
 * needs to live in this source file. */
const MATCH_MARKER_RE = new RegExp(`[${String.fromCharCode(1)}${String.fromCharCode(2)}]`);

/** Splits an FTS5 snippet on the match markers into plain text and
 * highlighted segments, so match highlighting never touches innerHTML. */
function renderSnippet(snippet: string) {
  const parts = snippet.split(MATCH_MARKER_RE);
  return parts.map((part, i) =>
    i % 2 === 1 ? <mark key={i}>{part}</mark> : <span key={i}>{part}</span>,
  );
}

export function SearchPanel() {
  const { query, results, loading, setQuery, close } = useSearchStore();
  const openFile = useEditorStore((s) => s.openFile);
  const jumpToText = useEditorStore((s) => s.jumpToText);
  const workspacePath = useWorkspaceStore((s) => s.path);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleSelect = async (result: SearchResult) => {
    if (!workspacePath) return;
    await openFile(result.path, workspacePath);
    jumpToText(query);
  };

  return (
    <div className="search-panel">
      <div className="search-panel__input-row">
        <input
          ref={inputRef}
          className="search-panel__input"
          type="text"
          value={query}
          placeholder="Search workspace…"
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") close();
          }}
        />
      </div>
      <div className="search-panel__results">
        {loading && <p className="sidebar__empty">Searching…</p>}
        {!loading && query.trim() !== "" && results.length === 0 && (
          <p className="sidebar__empty">No matches.</p>
        )}
        {!loading &&
          results.map((result) => (
            <button
              key={result.path}
              className="search-panel__result"
              onClick={() => handleSelect(result)}
              title={result.path}
            >
              <span className="search-panel__result-name">{result.name}</span>
              <span className="search-panel__result-snippet">{renderSnippet(result.snippet)}</span>
            </button>
          ))}
      </div>
    </div>
  );
}
