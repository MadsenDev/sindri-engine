import { useState, useEffect } from "react";
import Hierarchy from "../components/Hierarchy";
import type { EntityInfo } from "../app/types";

interface HierarchyPanelProps {
  entities: EntityInfo[];
  selectedEntityId: number | null;
  onDuplicate: () => void;
  onDelete: () => void;
  onSavePrefab: () => void;
  onInstantiatePrefab: () => void;
  onEntityClick: (entityId: number) => void;
  onRename: (entityId: number, name: string) => void;
  onContextMenuOpen: (screen: { x: number; y: number }) => void;
  onReparent: (entityId: number, parentId: number | null) => void;
}

export default function HierarchyPanel({
  entities,
  selectedEntityId,
  onDuplicate,
  onDelete,
  onSavePrefab,
  onInstantiatePrefab,
  onEntityClick,
  onRename,
  onContextMenuOpen,
  onReparent,
}: HierarchyPanelProps) {
  const [prefabMenuOpen, setPrefabMenuOpen] = useState(false);
  const hasSelection = selectedEntityId !== null;

  useEffect(() => {
    if (!prefabMenuOpen) return;
    const close = () => setPrefabMenuOpen(false);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [prefabMenuOpen]);

  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          <div className="create-menu">
            <button
              className="unity-button muted"
              onClick={(e) => {
                e.stopPropagation();
                setPrefabMenuOpen((p) => !p);
              }}
            >
              Prefabs ▾
            </button>
            {prefabMenuOpen && (
              <div className="create-menu-list" onClick={() => setPrefabMenuOpen(false)}>
                <button className="create-menu-item" onClick={onSavePrefab}>
                  Save Prefab
                </button>
                <button className="create-menu-item" onClick={onInstantiatePrefab}>
                  Add Prefab
                </button>
              </div>
            )}
          </div>
          <button
            className="unity-button muted"
            onClick={onDuplicate}
            disabled={!hasSelection}
          >
            Duplicate
          </button>
          <button
            className="unity-button danger"
            onClick={onDelete}
            disabled={!hasSelection}
          >
            Delete
          </button>
        </div>
      </div>
      <div className="panel-body muted-bg" style={{ overflow: "hidden", padding: 0 }}>
        <Hierarchy
          entities={entities}
          selectedEntityId={selectedEntityId}
          onEntityClick={onEntityClick}
          onRename={onRename}
          onContextMenuOpen={onContextMenuOpen}
          onReparent={onReparent}
        />
      </div>
    </div>
  );
}
