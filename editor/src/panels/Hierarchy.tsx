import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Scene, Entity, Component } from "../App";
import { useContextMenu } from "../components/ContextMenu";

interface Props {
  scene: Scene | null;
  selectedId: number | null;
  selectedComponent: number | null; // index in entity.components
  onSelect: (id: number) => void;
  onSelectComponent: (entityId: number, componentIdx: number) => void;
  onSceneChange: () => void;
  onDeleteEntity: (id: number) => Promise<void>;
}

const COMPONENT_TYPES = ["Transform", "Sprite", "PhysicsBody", "Collider", "Script", "Camera", "AudioSource"];

const COMPONENT_ICON: Record<string, string> = {
  Transform:   "⌖",
  Sprite:      "▣",
  PhysicsBody: "●",
  Collider:    "⬡",
  Script:      "⚡",
  Camera:      "◉",
  AudioSource: "♪",
};

interface CreatePreset {
  label: string;
  icon: string;
  name: string;
  transform?: { x: number; y: number; scale_x?: number; scale_y?: number; rotation?: number };
  components?: {
    type: Exclude<Component["type"], "Transform">;
    data?: Record<string, unknown>;
  }[];
}

const CREATE_PRESETS: CreatePreset[] = [
  {
    label: "Camera",
    icon: "◉",
    name: "Main Camera",
    transform: { x: 0, y: 0 },
    components: [{ type: "Camera", data: { active: true, zoom: 1.0, follow_entity: null, offset_x: 0.0, offset_y: 0.0, smoothing: 1.0, dead_zone_width: 0.0, dead_zone_height: 0.0 } }],
  },
  {
    label: "Sprite",
    icon: "▣",
    name: "Sprite",
    transform: { x: 0, y: 0 },
    components: [{ type: "Sprite", data: { texture_path: "generated:white", width: 64, height: 64, color: [0.34, 0.54, 1.0, 1.0] } }],
  },
  {
    label: "Physics Body",
    icon: "●",
    name: "Physics Body",
    transform: { x: 0, y: 0 },
    components: [
      { type: "Sprite", data: { texture_path: "generated:white", width: 48, height: 48, color: [0.0, 1.0, 0.9, 0.85] } },
      { type: "PhysicsBody", data: { body_type: "Dynamic", lock_rotation: true } },
      { type: "Collider", data: { width: 48, height: 48, is_trigger: false } },
    ],
  },
  {
    label: "Ground Platform",
    icon: "▭",
    name: "Ground",
    transform: { x: 0, y: 160 },
    components: [
      { type: "Sprite", data: { texture_path: "generated:white", width: 360, height: 32, color: [0.18, 0.22, 0.28, 1.0] } },
      { type: "PhysicsBody", data: { body_type: "Fixed" } },
      { type: "Collider", data: { width: 360, height: 32, is_trigger: false } },
    ],
  },
  {
    label: "Trigger Zone",
    icon: "⬡",
    name: "Trigger",
    transform: { x: 0, y: 0 },
    components: [
      { type: "Sprite", data: { texture_path: "generated:white", width: 96, height: 64, color: [0.5, 0.35, 1.0, 0.25] } },
      { type: "Collider", data: { width: 96, height: 64, is_trigger: true } },
    ],
  },
  {
    label: "Scripted Entity",
    icon: "⚡",
    name: "Scripted Entity",
    transform: { x: 0, y: 0 },
    components: [{ type: "Script", data: { path: "scripts/entity.lua" } }],
  },
  {
    label: "Audio Source",
    icon: "♪",
    name: "Audio Source",
    transform: { x: 0, y: 0 },
    components: [{ type: "AudioSource", data: { path: "", volume: 1.0, looping: false, play_on_start: false } }],
  },
];

export default function Hierarchy({ scene, selectedId, selectedComponent, onSelect, onSelectComponent, onSceneChange, onDeleteEntity }: Props) {
  const entities = scene ? scene.entities : {};
  const [renamingId, setRenamingId] = useState<number | null>(null);
  const { show } = useContextMenu();

  const roots = Object.values(entities)
    .filter(e => e.parent === null)
    .sort((a, b) => a.id - b.id);

  const handleAddEntity = async (parentId: number | null = null) => {
    try {
      await invoke("create_entity", { name: "Entity", parentId });
      onSceneChange();
    } catch {}
  };

  const handleCreatePreset = async (preset: CreatePreset, parentId: number | null = null) => {
    try {
      const created = await invoke<{ id: number }>("create_entity", { name: preset.name, parentId });
      const entityId = Number(created.id);
      let componentIdx = 0;

      if (preset.transform) {
        await invoke("patch_transform", {
          entityId,
          x: preset.transform.x,
          y: preset.transform.y,
          scaleX: preset.transform.scale_x ?? 1.0,
          scaleY: preset.transform.scale_y ?? 1.0,
          rotation: preset.transform.rotation ?? 0.0,
        });
        componentIdx = 1;
      }

      for (const component of preset.components ?? []) {
        await invoke("add_component", { entityId, componentType: component.type });
        if (component.data) {
          await invoke("patch_component", { entityId, componentIdx, data: component.data });
        }
        componentIdx += 1;
      }

      onSelect(entityId);
      onSceneChange();
    } catch (err) {
      console.error("create preset failed:", err);
    }
  };

  const createMenuItems = (parentId: number | null) => [
    { label: "Empty Entity", icon: "◻", onClick: () => handleAddEntity(parentId) },
    { divider: true as const },
    ...CREATE_PRESETS.map(preset => ({
      label: preset.label,
      icon: preset.icon,
      onClick: () => handleCreatePreset(preset, parentId),
    })),
  ];

  const handleAddComponent = async (entityId: number, componentType: string) => {
    try {
      await invoke("add_component", { entityId, componentType });
      onSceneChange();
    } catch (err) {
      console.error("add_component failed:", err);
    }
  };

  const handleRemoveComponent = async (entityId: number, componentIdx: number) => {
    try {
      await invoke("remove_component", { entityId, componentIdx });
      onSceneChange();
    } catch (err) {
      console.error("remove_component failed:", err);
    }
  };

  const handleRename = async (entityId: number, name: string) => {
    try {
      await invoke("rename_entity", { entityId, name });
      onSceneChange();
    } catch {}
  };

  const handleEntityContextMenu = (e: React.MouseEvent, entity: Entity) => {
    e.preventDefault();
    e.stopPropagation();
    const existingTypes = new Set(entity.components.map(c => c.type));
    show(e.clientX, e.clientY, [
      {
        label: "Add Component",
        icon: "⊕",
        children: COMPONENT_TYPES.map(ct => ({
          label: ct,
          icon: COMPONENT_ICON[ct],
          disabled: existingTypes.has(ct as Component["type"]),
          onClick: () => handleAddComponent(entity.id, ct),
        })),
      },
      { label: "Create Child", icon: "◻", children: createMenuItems(entity.id) },
      { divider: true as const },
      { label: "Rename", icon: "✎", onClick: () => { onSelect(entity.id); setRenamingId(entity.id); } },
      { label: "Delete", icon: "×", danger: true, onClick: () => onDeleteEntity(entity.id) },
    ]);
  };

  const handleComponentContextMenu = (e: React.MouseEvent, entity: Entity, componentIdx: number, componentType: string) => {
    e.preventDefault();
    e.stopPropagation();
    show(e.clientX, e.clientY, [
      { label: `Remove ${componentType}`, icon: "×", danger: true, onClick: () => handleRemoveComponent(entity.id, componentIdx) },
    ]);
  };

  const handleTreeContextMenu = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("[data-entity-item]")) return;
    e.preventDefault();
    show(e.clientX, e.clientY, createMenuItems(null));
  };


  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden", minHeight: 0 }}>
      {/* Scene section header */}
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "0 8px", height: "28px", borderBottom: "1px solid var(--border)", flexShrink: 0,
      }}>
        <span style={{
          fontFamily: "var(--font-ui)", fontWeight: 600, fontSize: "9px",
          color: "var(--text-dim)", letterSpacing: "0.12em", textTransform: "uppercase",
        }}>Scene</span>
        <button onClick={() => handleAddEntity(null)} style={{
          background: "none", border: "none", color: "var(--text-muted)",
          fontSize: "14px", cursor: "pointer", lineHeight: 1, padding: "0 2px",
        }}>+</button>
      </div>

      {/* Tree */}
      <div
        style={{ flex: 1, overflow: "auto", padding: "4px 0" }}
        onContextMenu={handleTreeContextMenu}
      >
        {roots.length === 0 && (
          <div style={{ padding: "8px 10px", color: "var(--text-dim)", fontSize: "11px" }}>
            No entities. Press + to add one.
          </div>
        )}
        {roots.map(entity => (
          <EntityTreeItem
            key={entity.id}
            entity={entity}
            entities={entities}
            selectedId={selectedId}
            selectedComponent={selectedComponent}
            onSelect={onSelect}
            onSelectComponent={onSelectComponent}
            onDelete={onDeleteEntity}
            onEntityContextMenu={handleEntityContextMenu}
            onComponentContextMenu={handleComponentContextMenu}
            isRenaming={renamingId === entity.id}
            onRenameCommit={async name => { await handleRename(entity.id, name); setRenamingId(null); }}
            onRenameCancel={() => setRenamingId(null)}
            renamingId={renamingId}
            onStartRename={setRenamingId}
            onRenameAny={handleRename}
            depth={0}
            isLast={roots[roots.length - 1].id === entity.id}
            parentLines={[]}
          />
        ))}
      </div>

    </div>
  );
}

// ─── Tree item props ─────────────────────────────────────────────────────────

interface TreeItemProps {
  entity: Entity;
  entities: Record<string, Entity>;
  selectedId: number | null;
  selectedComponent: number | null;
  onSelect: (id: number) => void;
  onSelectComponent: (entityId: number, componentIdx: number) => void;
  onDelete: (id: number) => Promise<void>;
  onEntityContextMenu: (e: React.MouseEvent, entity: Entity) => void;
  onComponentContextMenu: (e: React.MouseEvent, entity: Entity, componentIdx: number, componentType: string) => void;
  isRenaming: boolean;
  onRenameCommit: (name: string) => Promise<void>;
  onRenameCancel: () => void;
  renamingId: number | null;
  onStartRename: (id: number | null) => void;
  onRenameAny: (id: number, name: string) => Promise<void>;
  depth: number;
  isLast: boolean;
  parentLines: boolean[];
}

// ─── Entity node ─────────────────────────────────────────────────────────────

function EntityTreeItem({
  entity, entities, selectedId, selectedComponent, onSelect, onSelectComponent, onDelete,
  onEntityContextMenu, onComponentContextMenu,
  isRenaming, onRenameCommit, onRenameCancel,
  renamingId, onStartRename, onRenameAny,
  depth, isLast, parentLines,
}: TreeItemProps) {
  const [hovered, setHovered] = useState(false);
  const [renameValue, setRenameValue] = useState(entity.name);
  const renameRef = useRef<HTMLInputElement>(null);
  const isSelected = selectedId === entity.id;

  const entityChildren = entity.children
    .map(cid => entities[String(cid)])
    .filter(Boolean)
    .sort((a, b) => a.id - b.id);

  const hasChildren = entityChildren.length > 0 || entity.components.length > 0;
  // Total child rows = components + entity children (for isLast calculations)
  const totalChildren = entity.components.length + entityChildren.length;

  useEffect(() => {
    if (!isRenaming) setRenameValue(entity.name);
  }, [entity.name, isRenaming]);

  useEffect(() => {
    if (isRenaming && renameRef.current) {
      renameRef.current.focus();
      renameRef.current.select();
    }
  }, [isRenaming]);

  const commitRename = () => {
    const trimmed = renameValue.trim();
    if (trimmed && trimmed !== entity.name) onRenameCommit(trimmed);
    else onRenameCancel();
  };

  return (
    <>
      <div
        data-entity-item="1"
        onClick={() => !isRenaming && onSelect(entity.id)}
        onContextMenu={e => onEntityContextMenu(e, entity)}
        onDoubleClick={() => { onSelect(entity.id); onStartRename(entity.id); }}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        style={{
          position: "relative", display: "flex", alignItems: "center",
          height: "24px", paddingRight: "4px", cursor: "pointer",
          background: isSelected ? "var(--accent-glow)" : hovered ? "var(--bg-3)" : "transparent",
          userSelect: "none",
        }}
      >
        {isSelected && (
          <div style={{ position: "absolute", left: 0, top: 0, bottom: 0, width: "2px", background: "var(--accent)" }} />
        )}

        <TreeConnectors depth={depth} isLast={isLast} parentLines={parentLines} />

        <span style={{ width: "12px", fontSize: "9px", color: "var(--text-dim)", textAlign: "center", flexShrink: 0 }}>
          {hasChildren ? "▾" : ""}
        </span>
        <span style={{ fontSize: "11px", marginRight: "4px", flexShrink: 0 }}>
          {hasChildren ? "📁" : "◻"}
        </span>

        {isRenaming ? (
          <input
            ref={renameRef}
            value={renameValue}
            onChange={e => setRenameValue(e.target.value)}
            onBlur={commitRename}
            onKeyDown={e => {
              if (e.key === "Enter") { e.preventDefault(); commitRename(); }
              if (e.key === "Escape") { e.preventDefault(); onRenameCancel(); }
            }}
            onClick={e => e.stopPropagation()}
            style={{
              flex: 1, height: "18px", fontSize: "11px", padding: "0 4px",
              background: "var(--bg-0)", border: "1px solid var(--accent)",
              borderRadius: "3px", color: "var(--text-white)", outline: "none",
              fontFamily: "var(--font-mono)",
            }}
          />
        ) : (
          <span style={{
            fontSize: "11px", flex: 1,
            color: depth === 0 ? "var(--text-white)" : "var(--text-bright)",
            overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
          }}>
            {entity.name}
          </span>
        )}

        {hovered && !isRenaming && (
          <button
            onClick={e => { e.stopPropagation(); onDelete(entity.id); }}
            title="Delete entity"
            style={{
              background: "none", border: "none", color: "var(--text-muted)",
              fontSize: "12px", cursor: "pointer", padding: "0 4px",
              lineHeight: 1, flexShrink: 0, opacity: 0.7,
            }}
          >×</button>
        )}
      </div>

      {/* Component child nodes */}
      {entity.components.map((component, idx) => {
        const isLastComponent = idx === entity.components.length - 1 && entityChildren.length === 0;
        const isSelectedComp = selectedId === entity.id && selectedComponent === idx;
        return (
          <ComponentTreeItem
            key={`${component.type}-${idx}`}
            component={component}
            componentIdx={idx}
            entity={entity}
            isSelected={isSelectedComp}
            onSelect={onSelectComponent}
            onContextMenu={onComponentContextMenu}
            depth={depth + 1}
            isLast={isLastComponent}
            parentLines={[...parentLines, depth > 0 ? !isLast : false]}
          />
        );
      })}

      {/* Entity child nodes */}
      {entityChildren.map((child, idx) => {
        const childIdx = entity.components.length + idx;
        return (
          <EntityTreeItem
            key={child.id}
            entity={child}
            entities={entities}
            selectedId={selectedId}
            selectedComponent={selectedComponent}
            onSelect={onSelect}
            onSelectComponent={onSelectComponent}
            onDelete={onDelete}
            onEntityContextMenu={onEntityContextMenu}
            onComponentContextMenu={onComponentContextMenu}
            isRenaming={renamingId === child.id}
            onRenameCommit={async name => { await onRenameAny(child.id, name); onStartRename(null); }}
            onRenameCancel={() => onStartRename(null)}
            renamingId={renamingId}
            onStartRename={onStartRename}
            onRenameAny={onRenameAny}
            depth={depth + 1}
            isLast={childIdx === totalChildren - 1}
            parentLines={[...parentLines, depth > 0 ? !isLast : false]}
          />
        );
      })}
    </>
  );
}

// ─── Component node ───────────────────────────────────────────────────────────

interface ComponentTreeItemProps {
  component: Component;
  componentIdx: number;
  entity: Entity;
  isSelected: boolean;
  onSelect: (entityId: number, componentIdx: number) => void;
  onContextMenu: (e: React.MouseEvent, entity: Entity, componentIdx: number, componentType: string) => void;
  depth: number;
  isLast: boolean;
  parentLines: boolean[];
}

function ComponentTreeItem({ component, componentIdx, entity, isSelected, onSelect, onContextMenu, depth, isLast, parentLines }: ComponentTreeItemProps) {
  const [hovered, setHovered] = useState(false);
  const icon = COMPONENT_ICON[component.type] ?? "·";

  // Label: show script filename if available, otherwise just type name
  const label = component.type === "Script" && component.path
    ? `Script · ${component.path.split("/").pop()}`
    : component.type;

  return (
    <div
      data-entity-item="1"
      onClick={() => onSelect(entity.id, componentIdx)}
      onContextMenu={e => onContextMenu(e, entity, componentIdx, component.type)}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        position: "relative", display: "flex", alignItems: "center",
        height: "22px", paddingRight: "4px", cursor: "pointer",
        background: isSelected ? "var(--accent-glow)" : hovered ? "var(--bg-3)" : "transparent",
        userSelect: "none",
      }}
    >
      {isSelected && (
        <div style={{ position: "absolute", left: 0, top: 0, bottom: 0, width: "2px", background: "var(--accent)" }} />
      )}
      <TreeConnectors depth={depth} isLast={isLast} parentLines={parentLines} />

      <span style={{ width: "12px", flexShrink: 0 }} />
      <span style={{
        fontSize: "10px", marginRight: "5px", flexShrink: 0,
        color: isSelected ? "var(--accent)" : "var(--text-dim)",
        width: "12px", textAlign: "center",
      }}>{icon}</span>
      <span style={{
        fontSize: "10px", flex: 1,
        color: isSelected ? "var(--text-bright)" : "var(--text-muted)",
        overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
        fontFamily: "var(--font-mono)",
      }}>
        {label}
      </span>
    </div>
  );
}

// ─── Shared connector lines ───────────────────────────────────────────────────

function TreeConnectors({ depth, isLast, parentLines }: { depth: number; isLast: boolean; parentLines: boolean[] }) {
  return (
    <div style={{ display: "flex", flexShrink: 0 }}>
      {parentLines.map((hasLine, i) => (
        <div key={i} style={{ position: "relative", width: "16px", height: "24px", flexShrink: 0 }}>
          {hasLine && (
            <div style={{ position: "absolute", left: "7px", top: 0, bottom: 0, width: "1px", background: "var(--border-bright)" }} />
          )}
        </div>
      ))}
      {depth > 0 && (
        <div style={{ position: "relative", width: "16px", height: "24px", flexShrink: 0 }}>
          <div style={{ position: "absolute", left: "7px", top: 0, bottom: isLast ? "50%" : 0, width: "1px", background: "var(--border-bright)" }} />
          <div style={{ position: "absolute", left: "7px", top: "50%", width: "6px", height: "1px", background: "var(--border-bright)" }} />
        </div>
      )}
    </div>
  );
}
