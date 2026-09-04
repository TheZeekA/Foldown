import "./Toolbar.css";
import { useEditorStore, type ViewMode } from "../../stores/editor";
import * as cmd from "../../editor/commands";
import type { EditorView } from "@codemirror/view";
import { openMdGuideWindow } from "../../lib/mdGuideWindow";

function Icon({ children }: { children: React.ReactNode }) {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 18 18"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {children}
    </svg>
  );
}

const icons = {
  bold: (
    <Icon>
      <path d="M5 3h4.5a2.5 2.5 0 0 1 0 5H5V3z" />
      <path d="M5 8h5a2.5 2.5 0 0 1 0 7H5V8z" />
    </Icon>
  ),
  italic: (
    <Icon>
      <line x1="10" y1="3" x2="7" y2="15" />
      <line x1="6" y1="15" x2="9" y2="15" />
      <line x1="8" y1="3" x2="11" y2="3" />
    </Icon>
  ),
  heading: (
    <Icon>
      <path d="M4 3v12" />
      <path d="M13 3v12" />
      <path d="M4 9h9" />
    </Icon>
  ),
  strikethrough: (
    <Icon>
      <path d="M5 5c0-1.2 1.3-2 3-2s3 .8 3 2" />
      <path d="M6 13c0 1.2 1.3 2 3 2s3-.8 3-2" />
      <line x1="3" y1="9" x2="15" y2="9" />
    </Icon>
  ),
  bulletList: (
    <Icon>
      <line x1="4" y1="5" x2="4.01" y2="5" />
      <line x1="7" y1="5" x2="15" y2="5" />
      <line x1="4" y1="9" x2="4.01" y2="9" />
      <line x1="7" y1="9" x2="15" y2="9" />
      <line x1="4" y1="13" x2="4.01" y2="13" />
      <line x1="7" y1="13" x2="15" y2="13" />
    </Icon>
  ),
  orderedList: (
    <Icon>
      <text x="1.5" y="6.5" fontSize="5" stroke="none" fill="currentColor">1.</text>
      <line x1="7" y1="5" x2="15" y2="5" />
      <text x="1.5" y="10.5" fontSize="5" stroke="none" fill="currentColor">2.</text>
      <line x1="7" y1="9" x2="15" y2="9" />
      <text x="1.5" y="14.5" fontSize="5" stroke="none" fill="currentColor">3.</text>
      <line x1="7" y1="13" x2="15" y2="13" />
    </Icon>
  ),
  taskList: (
    <Icon>
      <rect x="3" y="3.5" width="4" height="4" rx="1" />
      <path d="M3.8 5.5l0.8 0.8L6.2 4.5" />
      <line x1="10" y1="5.5" x2="15" y2="5.5" />
      <rect x="3" y="10.5" width="4" height="4" rx="1" />
      <line x1="10" y1="12.5" x2="15" y2="12.5" />
    </Icon>
  ),
  blockquote: (
    <Icon>
      <path d="M5 5.5c-1.4 0-2.3.9-2.3 2.4S3.6 10.5 5 10.5" />
      <path d="M11 5.5c-1.4 0-2.3.9-2.3 2.4S9.6 10.5 11 10.5" />
    </Icon>
  ),
  code: (
    <Icon>
      <polyline points="6.5,4 2.5,9 6.5,14" />
      <polyline points="11.5,4 15.5,9 11.5,14" />
    </Icon>
  ),
  table: (
    <Icon>
      <rect x="2" y="3" width="14" height="12" rx="1" />
      <line x1="2" y1="7.5" x2="16" y2="7.5" />
      <line x1="2" y1="11.5" x2="16" y2="11.5" />
      <line x1="9" y1="3" x2="9" y2="15" />
    </Icon>
  ),
  link: (
    <Icon>
      <path d="M7.5 11a3 3 0 0 1 0-4.2l1.8-1.8a3 3 0 0 1 4.2 4.2l-.9.9" />
      <path d="M10.5 7a3 3 0 0 1 0 4.2l-1.8 1.8a3 3 0 0 1-4.2-4.2l.9-.9" />
    </Icon>
  ),
  image: (
    <Icon>
      <rect x="2" y="3" width="14" height="12" rx="1.5" />
      <circle cx="6.5" cy="7.5" r="1.2" fill="currentColor" stroke="none" />
      <path d="M3 13.5l4-4 3 3 3.5-4.5 2.5 3.5" />
    </Icon>
  ),
  source: (
    <Icon>
      <rect x="3" y="3" width="12" height="12" rx="1" />
      <line x1="5.5" y1="6.5" x2="12.5" y2="6.5" />
      <line x1="5.5" y1="9" x2="12.5" y2="9" />
      <line x1="5.5" y1="11.5" x2="9.5" y2="11.5" />
    </Icon>
  ),
  split: (
    <Icon>
      <rect x="2" y="3" width="14" height="12" rx="1" />
      <line x1="9" y1="3" x2="9" y2="15" />
    </Icon>
  ),
  preview: (
    <Icon>
      <path d="M2 9s2.7-5 7-5 7 5 7 5-2.7 5-7 5-7-5-7-5z" />
      <circle cx="9" cy="9" r="2" />
    </Icon>
  ),
};

interface ToolbarButtonProps {
  label: string;
  icon: React.ReactNode;
  onClick: () => void;
  disabled?: boolean;
  active?: boolean;
}

function ToolbarButton({ label, icon, onClick, disabled, active }: ToolbarButtonProps) {
  return (
    <button
      type="button"
      className={`toolbar__button${active ? " toolbar__button--active" : ""}`}
      title={label}
      aria-label={label}
      disabled={disabled}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
    >
      {icon}
    </button>
  );
}

function SaveStatus() {
  const dirty = useEditorStore((s) => s.dirty);
  const saveStatus = useEditorStore((s) => s.saveStatus);

  let label = "Saved";
  if (saveStatus === "error") label = "Save failed";
  else if (saveStatus === "saving") label = "Saving…";
  else if (dirty) label = "Unsaved";

  return <span className={`toolbar__save-status toolbar__save-status--${saveStatus}`}>{label}</span>;
}

export function Toolbar() {
  const view = useEditorStore((s) => s.view);
  const viewMode = useEditorStore((s) => s.viewMode);
  const setViewMode = useEditorStore((s) => s.setViewMode);

  const run = (fn: (view: EditorView) => void) => () => {
    if (view) fn(view);
  };

  const formattingDisabled = !view || viewMode === "preview";

  return (
    <div className="toolbar">
      <div className="toolbar__group">
        <ToolbarButton label="Bold" icon={icons.bold} disabled={formattingDisabled} onClick={run(cmd.toggleBold)} />
        <ToolbarButton label="Italic" icon={icons.italic} disabled={formattingDisabled} onClick={run(cmd.toggleItalic)} />
        <ToolbarButton label="Heading" icon={icons.heading} disabled={formattingDisabled} onClick={run(cmd.cycleHeading)} />
        <ToolbarButton label="Strikethrough" icon={icons.strikethrough} disabled={formattingDisabled} onClick={run(cmd.toggleStrikethrough)} />
      </div>
      <div className="toolbar__divider" />
      <div className="toolbar__group">
        <ToolbarButton label="Bullet list" icon={icons.bulletList} disabled={formattingDisabled} onClick={run(cmd.toggleBulletList)} />
        <ToolbarButton label="Numbered list" icon={icons.orderedList} disabled={formattingDisabled} onClick={run(cmd.toggleOrderedList)} />
        <ToolbarButton label="Task list" icon={icons.taskList} disabled={formattingDisabled} onClick={run(cmd.toggleTaskList)} />
        <ToolbarButton label="Blockquote" icon={icons.blockquote} disabled={formattingDisabled} onClick={run(cmd.toggleBlockquote)} />
      </div>
      <div className="toolbar__divider" />
      <div className="toolbar__group">
        <ToolbarButton label="Inline code" icon={icons.code} disabled={formattingDisabled} onClick={run(cmd.toggleInlineCode)} />
        <ToolbarButton label="Table" icon={icons.table} disabled={formattingDisabled} onClick={run(cmd.insertTable)} />
        <ToolbarButton label="Link" icon={icons.link} disabled={formattingDisabled} onClick={run(cmd.insertLink)} />
        <ToolbarButton label="Image" icon={icons.image} disabled={formattingDisabled} onClick={run(cmd.insertImage)} />
      </div>
      <div className="toolbar__spacer" />
      <SaveStatus />
      <div className="toolbar__group toolbar__group--modes">
        {(
          [
            ["source", "Source", icons.source],
            ["split", "Split", icons.split],
            ["preview", "Preview", icons.preview],
          ] as [ViewMode, string, React.ReactNode][]
        ).map(([mode, label, icon]) => (
          <ToolbarButton
            key={mode}
            label={label}
            icon={icon}
            active={viewMode === mode}
            onClick={() => setViewMode(mode)}
          />
        ))}
      </div>
      <button
        type="button"
        className="toolbar__md-guide-button"
        title="Open the Markdown cheat sheet"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => void openMdGuideWindow()}
      >
        MD Guide
      </button>
    </div>
  );
}
