import { useEffect, useState } from "react";
import type { TreeNode } from "../../lib/types";
import { useWorkspaceStore } from "../../stores/workspace";
import { useEditorStore } from "../../stores/editor";
import { useInteractiveModeStore } from "../../stores/interactiveMode";
import { NameInput } from "./NameInput";

interface TreeItemProps {
  node: TreeNode;
  depth: number;
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
}

export function TreeItem({ node, depth, onContextMenu }: TreeItemProps) {
  const [expanded, setExpanded] = useState(true);
  const [dragOver, setDragOver] = useState(false);
  const workspacePath = useWorkspaceStore((s) => s.path);
  const renamingPath = useWorkspaceStore((s) => s.renamingPath);
  const creating = useWorkspaceStore((s) => s.creating);
  const confirmRenaming = useWorkspaceStore((s) => s.confirmRenaming);
  const cancelRenaming = useWorkspaceStore((s) => s.cancelRenaming);
  const confirmCreating = useWorkspaceStore((s) => s.confirmCreating);
  const cancelCreating = useWorkspaceStore((s) => s.cancelCreating);
  const moveNode = useWorkspaceStore((s) => s.moveNode);
  const openPath = useEditorStore((s) => s.openPath);
  const openFile = useEditorStore((s) => s.openFile);
  const closeAi = useInteractiveModeStore((s) => s.close);

  const indent = { paddingLeft: `${depth * 1 + 0.5}rem` };
  const isFolder = node.type === "folder";
  const creatingHere = isFolder && creating?.parentPath === node.path;

  useEffect(() => {
    if (creatingHere) setExpanded(true);
  }, [creatingHere]);

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);
    const sourcePath = e.dataTransfer.getData("text/plain");
    if (sourcePath) moveNode(sourcePath, node.path);
  };

  if (renamingPath === node.path) {
    return (
      <div style={indent} className="tree-item__row">
        <NameInput initialValue={node.name} onConfirm={confirmRenaming} onCancel={cancelRenaming} />
      </div>
    );
  }

  if (isFolder) {
    return (
      <div>
        <button
          className={`tree-item__row tree-item__row--folder${dragOver ? " tree-item__row--drag-over" : ""}`}
          style={indent}
          onClick={() => setExpanded((value) => !value)}
          onContextMenu={(e) => onContextMenu(e, node)}
          draggable
          onDragStart={(e) => e.dataTransfer.setData("text/plain", node.path)}
          onDragOver={(e) => {
            e.preventDefault();
            setDragOver(true);
          }}
          onDragLeave={() => setDragOver(false)}
          onDrop={handleDrop}
        >
          <span className={`tree-item__caret${expanded ? " tree-item__caret--open" : ""}`}>
            ▸
          </span>
          <span className="tree-item__label">{node.name}</span>
        </button>
        {expanded && (
          <>
            {node.children.map((child) => (
              <TreeItem key={child.path} node={child} depth={depth + 1} onContextMenu={onContextMenu} />
            ))}
            {creatingHere && (
              <div style={{ paddingLeft: `${(depth + 1) * 1 + 0.5}rem` }} className="tree-item__row">
                <NameInput onConfirm={confirmCreating} onCancel={cancelCreating} />
              </div>
            )}
          </>
        )}
      </div>
    );
  }

  return (
    <div
      className={`tree-item__row tree-item__row--file${openPath === node.path ? " tree-item__row--active" : ""}`}
      style={indent}
      onClick={() => {
        closeAi();
        if (workspacePath) void openFile(node.path, workspacePath);
      }}
      onContextMenu={(e) => onContextMenu(e, node)}
      draggable
      onDragStart={(e) => e.dataTransfer.setData("text/plain", node.path)}
    >
      <span className="tree-item__label">{node.name}</span>
    </div>
  );
}
