import { useState } from "react";
import { useEditorStore } from "../../stores/editor";
import "./FrontmatterPanel.css";

function formatValue(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (Array.isArray(value)) return value.join(", ");
  if (value instanceof Date) return value.toISOString().slice(0, 10);
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

export function FrontmatterPanel() {
  const prefix = useEditorStore((s) => s.frontmatterPrefix);
  const data = useEditorStore((s) => s.frontmatterData);
  const error = useEditorStore((s) => s.frontmatterError);
  const [expanded, setExpanded] = useState(false);

  if (!prefix) return null;

  const fields = Object.entries(data);

  return (
    <div className="frontmatter">
      <button
        className="frontmatter__summary"
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
      >
        <span className={`frontmatter__caret${expanded ? " frontmatter__caret--open" : ""}`}>▸</span>
        Frontmatter{fields.length > 0 ? ` (${fields.length})` : ""}
      </button>
      {expanded && (
        <div className="frontmatter__body">
          {error && <p className="frontmatter__error">Couldn't parse this frontmatter: {error}</p>}
          {!error && fields.length === 0 && <p className="frontmatter__empty">No fields.</p>}
          {!error &&
            fields.map(([key, value]) => (
              <div key={key} className="frontmatter__field">
                <span className="frontmatter__key">{key}</span>
                <span className="frontmatter__value">{formatValue(value)}</span>
              </div>
            ))}
        </div>
      )}
    </div>
  );
}

export default FrontmatterPanel;
