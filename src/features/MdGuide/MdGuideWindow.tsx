import { useEffect } from "react";
import { useSettingsStore } from "../../stores/settings";
import "../../styles/theme.css";
import "./MdGuideWindow.css";

interface GuideEntry {
  title: string;
  syntax: string;
  result: React.ReactNode;
}

const ENTRIES: GuideEntry[] = [
  {
    title: "Headings",
    syntax: "# Heading 1\n## Heading 2\n### Heading 3",
    result: (
      <>
        <h1>Heading 1</h1>
        <h2>Heading 2</h2>
        <h3>Heading 3</h3>
      </>
    ),
  },
  { title: "Bold", syntax: "**bold text**", result: <strong>bold text</strong> },
  { title: "Italic", syntax: "*italic text*", result: <em>italic text</em> },
  { title: "Strikethrough", syntax: "~~strikethrough~~", result: <del>strikethrough</del> },
  {
    title: "Blockquote",
    syntax: "> A quoted passage.",
    result: <blockquote>A quoted passage.</blockquote>,
  },
  {
    title: "Bullet list",
    syntax: "- First item\n- Second item",
    result: (
      <ul>
        <li>First item</li>
        <li>Second item</li>
      </ul>
    ),
  },
  {
    title: "Numbered list",
    syntax: "1. First item\n2. Second item",
    result: (
      <ol>
        <li>First item</li>
        <li>Second item</li>
      </ol>
    ),
  },
  {
    title: "Task list",
    syntax: "- [ ] To do\n- [x] Done",
    result: (
      <ul className="md-guide__task-list">
        <li><input type="checkbox" disabled /> To do</li>
        <li><input type="checkbox" disabled defaultChecked /> Done</li>
      </ul>
    ),
  },
  { title: "Inline code", syntax: "`inline code`", result: <code>inline code</code> },
  {
    title: "Code block",
    syntax: "```\nconst x = 1;\n```",
    result: <pre><code>const x = 1;</code></pre>,
  },
  {
    title: "Link",
    syntax: "[Foldown](https://example.com)",
    result: <a href="#" onClick={(e) => e.preventDefault()}>Foldown</a>,
  },
  {
    title: "Image",
    syntax: "![Alt text](image.png)",
    result: <span className="md-guide__placeholder">🖼 image.png</span>,
  },
  {
    title: "Table",
    syntax: "| A | B |\n| - | - |\n| 1 | 2 |",
    result: (
      <table>
        <thead><tr><th>A</th><th>B</th></tr></thead>
        <tbody><tr><td>1</td><td>2</td></tr></tbody>
      </table>
    ),
  },
  { title: "Horizontal rule", syntax: "---", result: <hr /> },
];

export function MdGuideWindow() {
  const initSettings = useSettingsStore((s) => s.init);

  useEffect(() => {
    void initSettings();
  }, [initSettings]);

  return (
    <main className="md-guide">
      <header className="md-guide__header">
        <h1>Markdown Cheat Sheet</h1>
        <p>The formatting Foldown understands, and how to write it.</p>
      </header>
      <div className="md-guide__entries">
        {ENTRIES.map((entry) => (
          <section className="md-guide__entry" key={entry.title}>
            <h2 className="md-guide__entry-title">{entry.title}</h2>
            <div className="md-guide__entry-body">
              <pre className="md-guide__syntax"><code>{entry.syntax}</code></pre>
              <div className="md-guide__result">{entry.result}</div>
            </div>
          </section>
        ))}
      </div>
    </main>
  );
}
