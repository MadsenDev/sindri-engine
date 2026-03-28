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
  onContextMenuOpen,
  onReparent,
}: HierarchyPanelProps) {
  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          <button className="unity-button muted" onClick={onSavePrefab}>
            Save Prefab
          </button>
          <button className="unity-button muted" onClick={onInstantiatePrefab}>
            Add Prefab
          </button>
          <button className="unity-button muted" onClick={onDuplicate}>
            Duplicate
          </button>
          <button className="unity-button danger" onClick={onDelete}>
            Delete
          </button>
        </div>
      </div>
      <div className="panel-body muted-bg">
        <Hierarchy
          entities={entities}
          selectedEntityId={selectedEntityId}
          onEntityClick={onEntityClick}
          onContextMenuOpen={onContextMenuOpen}
          onReparent={onReparent}
        />
      </div>
    </div>
  );
}
