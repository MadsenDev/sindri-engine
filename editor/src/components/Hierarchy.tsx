import { useState } from "react";

interface EntityInfo {
  id: number;
  has_transform: boolean;
  has_sprite: boolean;
  has_physics: boolean;
  has_camera: boolean;
  parent_id: number | null;
  children: number[];
}

interface HierarchyProps {
  entities: EntityInfo[];
  selectedEntityId: number | null;
  onEntityClick: (entityId: number) => void;
  onContextMenuOpen?: (screen: { x: number; y: number }) => void;
}

function HierarchyNode({
  entity,
  entities,
  selectedEntityId,
  onEntityClick,
  level = 0,
}: {
  entity: EntityInfo;
  entities: EntityInfo[];
  selectedEntityId: number | null;
  onEntityClick: (entityId: number) => void;
  level?: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const hasChildren = entity.children.length > 0;
  const isSelected = selectedEntityId === entity.id;

  const childEntities = entity.children
    .map((childId) => entities.find((e) => e.id === childId))
    .filter((e): e is EntityInfo => e !== undefined);

  return (
    <div>
      <div
        className={`hierarchy-item ${isSelected ? "selected" : ""}`}
        style={{ paddingLeft: `${level * 16 + 8}px` }}
        onClick={() => onEntityClick(entity.id)}
      >
        {hasChildren && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              setExpanded(!expanded);
            }}
            className="collapse-toggle"
            aria-label={expanded ? "Collapse" : "Expand"}
          >
            {expanded ? "▼" : "▶"}
          </button>
        )}
        {!hasChildren && <span className="collapse-placeholder" />}
        <div className="flex flex-col gap-1">
          <span className="font-mono text-sm">Entity {entity.id}</span>
          <div className="flex items-center gap-1 text-[11px] text-gray-400">
            {entity.has_transform && <span className="component-tag">Transform</span>}
            {entity.has_sprite && <span className="component-tag">Sprite</span>}
            {entity.has_physics && <span className="component-tag">Physics</span>}
            {entity.has_camera && <span className="component-tag">Camera</span>}
            {!entity.has_transform && !entity.has_sprite && !entity.has_physics && !entity.has_camera && (
              <span className="text-gray-500">Empty</span>
            )}
          </div>
        </div>
      </div>
      {hasChildren && expanded && (
        <div>
          {childEntities.map((child) => (
            <HierarchyNode
              key={child.id}
              entity={child}
              entities={entities}
              selectedEntityId={selectedEntityId}
              onEntityClick={onEntityClick}
              level={level + 1}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export default function Hierarchy({
  entities,
  selectedEntityId,
  onEntityClick,
  onContextMenuOpen,
}: HierarchyProps) {
  // Find root entities (those with no parent)
  const rootEntities = entities.filter((e) => e.parent_id === null);

  return (
    <div
      className="h-full overflow-y-auto"
      onContextMenu={(e) => {
        if (!onContextMenuOpen) return;
        e.preventDefault();
        onContextMenuOpen({ x: e.clientX, y: e.clientY });
      }}
    >
      {rootEntities.length === 0 ? (
        <p className="text-gray-400 text-sm p-2">No entities</p>
      ) : (
        <div>
          {rootEntities.map((entity) => (
            <HierarchyNode
              key={entity.id}
              entity={entity}
              entities={entities}
              selectedEntityId={selectedEntityId}
              onEntityClick={onEntityClick}
            />
          ))}
        </div>
      )}
    </div>
  );
}
