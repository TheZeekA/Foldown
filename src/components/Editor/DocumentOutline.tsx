import { extractMarkdownHeadings } from "../../editor/outline";
import { useEditorStore } from "../../stores/editor";
import "./DocumentOutline.css";

export function DocumentOutline({ body }: { body: string }) {
  const jumpToPosition = useEditorStore((state) => state.jumpToPosition);
  const headings = extractMarkdownHeadings(body);

  return (
    <aside className="document-outline" aria-label="Document outline">
      <div className="document-outline__title">Outline</div>
      {headings.length === 0 ? (
        <p className="document-outline__empty">Add headings to see the document outline.</p>
      ) : (
        <nav>
          <ol className="document-outline__list">
            {headings.map((heading) => (
              <li key={`${heading.from}-${heading.level}`}>
                <button
                  type="button"
                  className="document-outline__item"
                  style={{ paddingLeft: `${0.55 + (heading.level - 1) * 0.7}rem` }}
                  title={`Jump to ${heading.text}`}
                  onClick={() => jumpToPosition(heading.from)}
                >
                  <span className="document-outline__level">H{heading.level}</span>
                  <span className="document-outline__text">{heading.text}</span>
                </button>
              </li>
            ))}
          </ol>
        </nav>
      )}
    </aside>
  );
}

