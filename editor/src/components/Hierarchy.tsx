import { useEffect, useRef, useState } from "react";
import type { EntityInfo } from "../app/types";

interface HierarchyProps {
  entities: EntityInfo[];
  selectedEntityId: number | null;
  onEntityClick: (entityId: number) => void;
  onRename: (entityId: number, name: string) => void;
  onContextMenuOpen?: (screen: { x: number; y: number }) => void;
  onReparent?: (entityId: number, parentId: number | null) => void;
}

function HierarchyNode({
  entity,
  entities,
  selectedEntityId,
  onEntityClick,
  onRename,
  onReparent,
  level = 0,
}: {
  entity: EntityInfo;
  entities: EntityInfo[];
  selectedEntityId: number | null;
  onEntityClick: (entityId: number) => void;
  onRename: (entityId: number, name: string) => void;
  onReparent?: (entityId: number, parentId: number | null) => void;
  level?: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const [isRenaming, setIsRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState(entity.name);
  const inputRef = useRef<HTMLInputElement>(null);

  const hasChildren = entity.children.length > 0;
  const isSelected = selectedEntityId === entity.id;

  useEffect(() => {
    if (isRenaming && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isRenaming]);

  const commitRename = () => {
    const trimmed = renameValue.trim();
    if (trimmed && trimmed !== entity.name) {
      onRename(entity.id, trimmed);
    } else {
      setRenameValue(entity.name);
    }
    setIsRenaming(false);
  };

  const childEntities = entity.children
    .map((childId) => entities.find((e) => e.id === childId))
    .filter((e): e is EntityInfo => e !== undefined);

  const componentTags: string[] = [];
  if (entity.has_transform) componentTags.push("T");
  if (entity.has_sprite) componentTags.push("S");
  if (entity.has_physics) componentTags.push("P");
  if (entity.has_camera) componentTags.push("C");

  return (
    <div>
      <div
        className={`hierarchy-item ${isSelected ? "selected" : ""}`}
        style={{ paddingLeft: `${level * 16 + 8}px` }}
        onClick={() => {
          if (!isRenaming) onEntityClick(entity.id);
        }}
        draggable={!isRenaming}
        onDragStart={(event) => {
          event.dataTransfer.setData("application/x-forge2d-entity", String(entity.id));
          event.dataTransfer.effectAllowed = "move";
        }}
        onDragOver={(event) => {
          event.preventDefault();
          event.dataTransfer.dropEffect = "move";
        }}
        onDrop={(event) => {
          event.preventDefault();
          const draggedId = Number(event.dataTransfer.getData("application/x-forge2d-entity"));
          if (!draggedId || draggedId === entity.id || !onReparent) {
            return;
          }
          onReparent(draggedId, entity.id);
        }}
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

        <div className="hierarchy-item-body">
          {isRenaming ? (
            <input
              ref={inputRef}
              className="hierarchy-rename-input"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={commitRename}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitRename();
                if (e.key === "Escape") {
                  setRenameValue(entity.name);
                  setIsRenaming(false);
                }
                e.stopPropagation();
              }}
              onClick={(e) => e.stopPropagation()}
            />
          ) : (
            <span
              className="hierarchy-item-name"
              onDoubleClick={(e) => {
                e.stopPropagation();
                setRenameValue(entity.name);
                setIsRenaming(true);
              }}
            >
              {entity.name}
            </span>
          )}
          {componentTags.length > 0 && (
            <span className="hierarchy-item-tags">{componentTags.join(" · ")}</span>
          )}
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
              onRename={onRename}
              onReparent={onReparent}
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
  onRename,
  onContextMenuOpen,
  onReparent,
}: HierarchyProps) {
  const rootEntities = entities.filter((e) => e.parent_id === null);

  return (
    <div
      className="hierarchy-scroll"
      onContextMenu={(e) => {
        if (!onContextMenuOpen) return;
        e.preventDefault();
        onContextMenuOpen({ x: e.clientX, y: e.clientY });
      }}
      onDragOver={(event) => {
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
      }}
      onDrop={(event) => {
        event.preventDefault();
        const draggedId = Number(event.dataTransfer.getData("application/x-forge2d-entity"));
        if (!draggedId || !onReparent) {
          return;
        }
        onReparent(draggedId, null);
      }}
    >
      {rootEntities.length === 0 ? (
        <p className="hierarchy-empty">No entities in scene</p>
      ) : (
        <div>
          {rootEntities.map((entity) => (
            <HierarchyNode
              key={entity.id}
              entity={entity}
              entities={entities}
              selectedEntityId={selectedEntityId}
              onEntityClick={onEntityClick}
              onRename={onRename}
              onReparent={onReparent}
            />
          ))}
        </div>
      )}
    </div>
  );
}
