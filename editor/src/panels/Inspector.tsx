import { useState, type CSSProperties, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Entity, Component } from "../App";

const COMPONENT_ICON: Record<string, string> = {
  Transform:   "⌖",
  Sprite:      "▣",
  PhysicsBody: "●",
  Collider:    "⬡",
  Script:      "⚡",
  Camera:      "◉",
  AudioSource: "♪",
};

interface Props {
  entity: Entity | null;
  selectedComponent: number | null; // index in entity.components
  onSelectComponent: (idx: number | null) => void;
  onSceneChange: () => void;
  onOpenScript: (path: string) => void;
}

export default function Inspector({ entity, selectedComponent, onSelectComponent, onSceneChange, onOpenScript }: Props) {
  if (!entity) {
    return (
      <div style={{ padding: "10px", color: "var(--text-dim)", fontSize: "11px", borderBottom: "1px solid var(--border)" }}>
        No entity selected
      </div>
    );
  }

  const comp = selectedComponent !== null
    ? entity.components[selectedComponent] ?? null
    : null;

  return (
    <div style={{ borderBottom: "1px solid var(--border)", flexShrink: 0, maxHeight: "45%", overflow: "auto" }}>
      {comp !== null && selectedComponent !== null
        ? <ComponentView key={`${entity.id}:${selectedComponent}:${comp.type}`} entity={entity} component={comp} componentIdx={selectedComponent} onBack={() => onSelectComponent(null)} onSceneChange={onSceneChange} onOpenScript={onOpenScript} />
        : <EntityOverview entity={entity} onSelectComponent={onSelectComponent} />
      }
    </div>
  );
}

// ─── Entity overview ─────────────────────────────────────────────────────────

function EntityOverview({ entity, onSelectComponent }: { entity: Entity; onSelectComponent: (idx: number) => void }) {
  return (
    <>
      <div style={{
        padding: "5px 10px", borderBottom: "1px solid var(--border)",
        display: "flex", alignItems: "center", gap: "6px",
        fontSize: "11px", color: "var(--text-white)", fontWeight: 500, flexShrink: 0,
      }}>
        <span style={{ flex: 1 }}>{entity.name}</span>
        <span style={{ color: "var(--text-dim)", fontSize: "10px" }}>#{entity.id}</span>
        <div style={{
          width: "7px", height: "7px", borderRadius: "50%",
          background: entity.active ? "var(--green)" : "var(--text-dim)",
        }} title={entity.active ? "active" : "inactive"} />
      </div>

      {entity.components.length === 0 ? (
        <div style={{ padding: "8px 10px", color: "var(--text-dim)", fontSize: "11px" }}>
          No components. Right-click entity to add one.
        </div>
      ) : (
        <div style={{ padding: "4px 6px", display: "flex", flexDirection: "column", gap: "2px" }}>
          {entity.components.map((comp, idx) => (
            <button
              key={`${comp.type}-${idx}`}
              onClick={() => onSelectComponent(idx)}
              style={{
                display: "flex", alignItems: "center", gap: "8px",
                height: "26px", padding: "0 8px",
                background: "var(--bg-3)", border: "1px solid var(--border-bright)",
                borderRadius: "var(--radius)", cursor: "pointer",
                textAlign: "left", width: "100%",
              }}
              onMouseEnter={e => (e.currentTarget as HTMLElement).style.borderColor = "var(--accent-dim)"}
              onMouseLeave={e => (e.currentTarget as HTMLElement).style.borderColor = "var(--border-bright)"}
            >
              <span style={{ fontSize: "11px", color: "var(--text-dim)", width: "14px", textAlign: "center", flexShrink: 0 }}>
                {COMPONENT_ICON[comp.type] ?? "·"}
              </span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--text-bright)", flex: 1 }}>
                {comp.type === "Script" && comp.path
                  ? `Script · ${comp.path.split("/").pop()}`
                  : comp.type}
              </span>
              <span style={{ fontSize: "9px", color: "var(--text-dim)" }}>▸</span>
            </button>
          ))}
        </div>
      )}
    </>
  );
}

// ─── Single component view ────────────────────────────────────────────────────

function ComponentView({ entity, component, componentIdx, onBack, onSceneChange, onOpenScript }: {
  entity: Entity;
  component: Component;
  componentIdx: number;
  onBack: () => void;
  onSceneChange: () => void;
  onOpenScript: (path: string) => void;
}) {
  const icon = COMPONENT_ICON[component.type] ?? "·";

  return (
    <>
      {/* Breadcrumb header */}
      <div style={{
        display: "flex", alignItems: "center", gap: "0",
        height: "28px", borderBottom: "1px solid var(--border)", flexShrink: 0,
      }}>
        <button
          onClick={onBack}
          title="Back to entity"
          style={{
            background: "none", border: "none", color: "var(--text-muted)",
            cursor: "pointer", padding: "0 8px", height: "100%",
            display: "flex", alignItems: "center", gap: "4px",
            fontSize: "11px", borderRight: "1px solid var(--border)",
          }}
          onMouseEnter={e => (e.currentTarget as HTMLElement).style.color = "var(--text-bright)"}
          onMouseLeave={e => (e.currentTarget as HTMLElement).style.color = "var(--text-muted)"}
        >
          ‹
        </button>
        <div style={{ display: "flex", alignItems: "center", gap: "6px", padding: "0 10px" }}>
          <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>{entity.name}</span>
          <span style={{ fontSize: "10px", color: "var(--border-bright)" }}>›</span>
          <span style={{ fontSize: "10px", color: "var(--text-dim)" }}>{icon}</span>
          <span style={{
            fontFamily: "var(--font-ui)", fontWeight: 600, fontSize: "9px",
            color: "var(--text-dim)", letterSpacing: "0.1em", textTransform: "uppercase",
          }}>{component.type}</span>
        </div>
      </div>

      {/* Component fields */}
      <div style={{ paddingBottom: "4px" }}>
        {component.type === "Transform" && (
          <TransformFields comp={component} entityId={entity.id} onSceneChange={onSceneChange} />
        )}
        {component.type === "Sprite"      && <SpriteFields comp={component} entityId={entity.id} componentIdx={componentIdx} onSceneChange={onSceneChange} />}
        {component.type === "PhysicsBody" && <PhysicsBodyFields comp={component} entityId={entity.id} componentIdx={componentIdx} onSceneChange={onSceneChange} />}
        {component.type === "Collider"    && <ColliderFields comp={component} entityId={entity.id} componentIdx={componentIdx} onSceneChange={onSceneChange} />}
        {component.type === "Script"      && <ScriptField comp={component} entityId={entity.id} componentIdx={componentIdx} onOpenScript={onOpenScript} onSceneChange={onSceneChange} />}
        {component.type === "Camera"      && <CameraFields comp={component} entityId={entity.id} componentIdx={componentIdx} onSceneChange={onSceneChange} />}
        {component.type === "AudioSource" && <AudioFields comp={component} entityId={entity.id} componentIdx={componentIdx} onSceneChange={onSceneChange} />}
      </div>
    </>
  );
}

// ─── Transform ───────────────────────────────────────────────────────────────

function TransformFields({ comp, entityId, onSceneChange }: {
  comp: Extract<Component, { type: "Transform" }>;
  entityId: number;
  onSceneChange: () => void;
}) {
  const [focused, setFocused] = useState<string | null>(null);

  const patch = async (field: string, value: string) => {
    const num = parseFloat(value);
    if (isNaN(num)) return;
    const map: Record<string, string> = { x: "x", y: "y", scale_x: "scaleX", scale_y: "scaleY", rotation: "rotation" };
    const args: Record<string, unknown> = { entityId, x: null, y: null, scaleX: null, scaleY: null, rotation: null };
    args[map[field]] = num;
    try {
      await invoke("patch_transform", args);
      onSceneChange();
    } catch {}
  };

  const rows: { label: string; fields: { key: string; value: number }[] }[] = [
    { label: "position", fields: [{ key: "x", value: comp.x }, { key: "y", value: comp.y }] },
    { label: "scale",    fields: [{ key: "scale_x", value: comp.scale_x }, { key: "scale_y", value: comp.scale_y }] },
    { label: "rotation", fields: [{ key: "rotation", value: comp.rotation }] },
  ];

  return (
    <>
      {rows.map(row => (
        <div key={row.label} style={{ display: "flex", alignItems: "center", height: "26px", padding: "0 10px", gap: "6px" }}>
          <span style={{ width: "60px", fontSize: "10px", color: "var(--text-dim)", flexShrink: 0 }}>{row.label}</span>
          {row.fields.map(f => (
            <input
              key={f.key}
              defaultValue={f.value.toFixed(2)}
              onFocus={() => setFocused(f.key)}
              onBlur={e => { setFocused(null); patch(f.key, e.target.value); }}
              onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
              style={{
                flex: 1, height: "18px", background: "var(--bg-3)",
                border: `1px solid ${focused === f.key ? "var(--accent)" : "var(--border-bright)"}`,
                borderRadius: "var(--radius)", color: "var(--text-bright)",
                fontFamily: "var(--font-mono)", fontSize: "11px", padding: "0 4px",
                outline: "none", minWidth: 0,
              }}
            />
          ))}
        </div>
      ))}
    </>
  );
}

// ─── Sprite ──────────────────────────────────────────────────────────────────

function SpriteFields({ comp, entityId, componentIdx, onSceneChange }: {
  comp: Extract<Component, { type: "Sprite" }>;
  entityId: number;
  componentIdx: number;
  onSceneChange: () => void;
}) {
  const patch = useComponentPatch(entityId, componentIdx, onSceneChange);

  return (
    <>
      <TextInputField label="texture" value={comp.texture_path} placeholder="(none)" onCommit={v => patch({ texture_path: v })} />
      <NumberInputField label="width" value={comp.width} onCommit={v => patch({ width: v })} />
      <NumberInputField label="height" value={comp.height} onCommit={v => patch({ height: v })} />
      <BoolField label="flip x" value={comp.flip_x} onChange={v => patch({ flip_x: v })} />
      <BoolField label="flip y" value={comp.flip_y} onChange={v => patch({ flip_y: v })} />
      <ColorField label="color" value={comp.color} onCommit={v => patch({ color: v })} />
    </>
  );
}

// ─── Physics Body ────────────────────────────────────────────────────────────

function PhysicsBodyFields({ comp, entityId, componentIdx, onSceneChange }: {
  comp: Extract<Component, { type: "PhysicsBody" }>;
  entityId: number;
  componentIdx: number;
  onSceneChange: () => void;
}) {
  const patch = useComponentPatch(entityId, componentIdx, onSceneChange);

  return (
    <>
      <div style={{ display: "flex", alignItems: "center", height: "26px", padding: "0 10px", gap: "6px" }}>
        <span style={{ width: "60px", fontSize: "10px", color: "var(--text-dim)", flexShrink: 0 }}>type</span>
        <select
          value={comp.body_type}
          onChange={e => patch({ body_type: e.currentTarget.value })}
          style={{
            flex: 1, height: "20px", background: "var(--bg-3)",
            border: "1px solid var(--border-bright)", color: "var(--text-bright)",
            fontFamily: "var(--font-mono)", fontSize: "10px",
          }}
        >
          <option>Dynamic</option>
          <option>Kinematic</option>
          <option>Fixed</option>
        </select>
      </div>
      <BoolField label="lock rot" value={comp.lock_rotation} onChange={v => patch({ lock_rotation: v })} />
      <NumberInputField label="lin damp" value={comp.linear_damping} onCommit={v => patch({ linear_damping: v })} />
      <NumberInputField label="ang damp" value={comp.angular_damping} onCommit={v => patch({ angular_damping: v })} />
      <NumberInputField label="layer" value={comp.collision_layer} decimals={0} min={0} max={255} onCommit={v => patch({ collision_layer: Math.round(v) })} />
      <NumberInputField label="mask" value={comp.collision_mask} decimals={0} min={0} onCommit={v => patch({ collision_mask: Math.round(v) })} />
    </>
  );
}

// ─── Collider ────────────────────────────────────────────────────────────────

function ColliderFields({ comp, entityId, componentIdx, onSceneChange }: {
  comp: Extract<Component, { type: "Collider" }>;
  entityId: number;
  componentIdx: number;
  onSceneChange: () => void;
}) {
  const patch = useComponentPatch(entityId, componentIdx, onSceneChange);

  return (
    <>
      <NumberInputField label="width" value={comp.width} onCommit={v => patch({ width: v })} />
      <NumberInputField label="height" value={comp.height} onCommit={v => patch({ height: v })} />
      <NumberInputField label="offset x" value={comp.offset_x} onCommit={v => patch({ offset_x: v })} />
      <NumberInputField label="offset y" value={comp.offset_y} onCommit={v => patch({ offset_y: v })} />
      <BoolField label="trigger" value={comp.is_trigger} onChange={v => patch({ is_trigger: v })} />
    </>
  );
}

function BoolField({ label, value, onChange }: { label: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <label style={{ display: "flex", alignItems: "center", height: "22px", padding: "0 10px", gap: "6px", cursor: "pointer" }}>
      <span style={{ width: "60px", fontSize: "10px", color: "var(--text-dim)", flexShrink: 0 }}>{label}</span>
      <input type="checkbox" checked={value} onChange={e => onChange(e.currentTarget.checked)} />
      <span style={{ fontSize: "10px", color: value ? "var(--accent)" : "var(--text-muted)" }}>{String(value)}</span>
    </label>
  );
}

// ─── Script ──────────────────────────────────────────────────────────────────

function ScriptField({ comp, entityId, componentIdx, onOpenScript, onSceneChange }: {
  comp: Extract<Component, { type: "Script" }>;
  entityId: number;
  componentIdx: number;
  onOpenScript: (path: string) => void;
  onSceneChange: () => void;
}) {
  const [focused, setFocused] = useState(false);

  const patchPath = async (newPath: string) => {
    try {
      await invoke("patch_component", { entityId, componentIdx, data: { path: newPath } });
      onSceneChange();
    } catch {}
  };

  return (
    <div style={{ padding: "2px 10px 6px" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "6px", height: "26px" }}>
        <span style={{ width: "60px", fontSize: "10px", color: "var(--text-dim)", flexShrink: 0 }}>path</span>
        <input
          defaultValue={comp.path || ""}
          placeholder="(none)"
          onFocus={() => setFocused(true)}
          onBlur={e => { setFocused(false); patchPath(e.target.value); }}
          onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
          style={{
            flex: 1, height: "18px", background: "var(--bg-3)",
            border: `1px solid ${focused ? "var(--accent)" : "var(--border-bright)"}`,
            borderRadius: "var(--radius)", color: "var(--text-bright)",
            fontFamily: "var(--font-mono)", fontSize: "10px", padding: "0 4px",
            outline: "none", minWidth: 0,
          }}
        />
      </div>
      <button
        onClick={() => comp.path && onOpenScript(comp.path)}
        disabled={!comp.path}
        style={{
          background: "var(--ai-glow)", border: "1px solid var(--ai-dim)",
          borderRadius: "var(--radius)", color: comp.path ? "var(--ai)" : "var(--text-dim)",
          fontFamily: "var(--font-mono)", fontSize: "10px",
          padding: "3px 10px", cursor: comp.path ? "pointer" : "default", width: "100%",
        }}
      >Open in editor</button>
    </div>
  );
}

// ─── Camera ──────────────────────────────────────────────────────────────────

function CameraFields({ comp, entityId, componentIdx, onSceneChange }: {
  comp: Extract<Component, { type: "Camera" }>;
  entityId: number;
  componentIdx: number;
  onSceneChange: () => void;
}) {
  const patch = useComponentPatch(entityId, componentIdx, onSceneChange);

  return (
    <>
      <BoolField label="active" value={comp.active ?? true} onChange={v => patch({ active: v })} />
      <NumberInputField label="zoom" value={comp.zoom} min={0.01} onCommit={v => patch({ zoom: v })} />
      <OptionalEntityField label="follow" value={comp.follow_entity} onCommit={v => patch({ follow_entity: v })} />
      <NumberInputField label="offset x" value={comp.offset_x ?? 0} onCommit={v => patch({ offset_x: v })} />
      <NumberInputField label="offset y" value={comp.offset_y ?? 0} onCommit={v => patch({ offset_y: v })} />
      <NumberInputField label="smoothing" value={comp.smoothing ?? 1} min={0} max={1} onCommit={v => patch({ smoothing: v })} />
      <NumberInputField label="dead w" value={comp.dead_zone_width ?? 0} min={0} onCommit={v => patch({ dead_zone_width: v })} />
      <NumberInputField label="dead h" value={comp.dead_zone_height ?? 0} min={0} onCommit={v => patch({ dead_zone_height: v })} />
      <OptionalNumberField label="min x" value={comp.bounds_min_x ?? null} onCommit={v => patch({ bounds_min_x: v })} />
      <OptionalNumberField label="min y" value={comp.bounds_min_y ?? null} onCommit={v => patch({ bounds_min_y: v })} />
      <OptionalNumberField label="max x" value={comp.bounds_max_x ?? null} onCommit={v => patch({ bounds_max_x: v })} />
      <OptionalNumberField label="max y" value={comp.bounds_max_y ?? null} onCommit={v => patch({ bounds_max_y: v })} />
    </>
  );
}

// ─── AudioSource ─────────────────────────────────────────────────────────────

function AudioFields({ comp, entityId, componentIdx, onSceneChange }: {
  comp: Extract<Component, { type: "AudioSource" }>;
  entityId: number;
  componentIdx: number;
  onSceneChange: () => void;
}) {
  const patch = useComponentPatch(entityId, componentIdx, onSceneChange);

  return (
    <>
      <TextInputField label="path" value={comp.path} placeholder="(none)" onCommit={v => patch({ path: v })} />
      <NumberInputField label="volume" value={comp.volume} min={0} onCommit={v => patch({ volume: v })} />
      <BoolField label="loop" value={comp.looping} onChange={v => patch({ looping: v })} />
      <BoolField label="autoplay" value={comp.play_on_start} onChange={v => patch({ play_on_start: v })} />
    </>
  );
}

// ─── Shared ───────────────────────────────────────────────────────────────────

function useComponentPatch(entityId: number, componentIdx: number, onSceneChange: () => void) {
  return async (data: Record<string, unknown>) => {
    try {
      await invoke("patch_component", { entityId, componentIdx, data });
      onSceneChange();
    } catch (err) {
      console.error("patch_component failed:", err);
    }
  };
}

function TextInputField({ label, value, placeholder, onCommit }: {
  label: string;
  value: string;
  placeholder?: string;
  onCommit: (value: string) => void;
}) {
  const [focused, setFocused] = useState(false);

  return (
    <EditableRow label={label}>
      <input
        defaultValue={value}
        placeholder={placeholder}
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); onCommit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

function NumberInputField({ label, value, decimals = 2, min, max, onCommit }: {
  label: string;
  value: number;
  decimals?: number;
  min?: number;
  max?: number;
  onCommit: (value: number) => void;
}) {
  const [focused, setFocused] = useState(false);

  const commit = (raw: string) => {
    let next = Number(raw);
    if (!Number.isFinite(next)) return;
    if (min !== undefined) next = Math.max(min, next);
    if (max !== undefined) next = Math.min(max, next);
    onCommit(next);
  };

  return (
    <EditableRow label={label}>
      <input
        type="number"
        defaultValue={decimals === 0 ? String(Math.round(value)) : value.toFixed(decimals)}
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); commit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

function OptionalEntityField({ label, value, onCommit }: {
  label: string;
  value: number | null;
  onCommit: (value: number | null) => void;
}) {
  const [focused, setFocused] = useState(false);

  const commit = (raw: string) => {
    const trimmed = raw.trim();
    if (trimmed === "") {
      onCommit(null);
      return;
    }
    const next = Number(trimmed);
    if (Number.isInteger(next) && next >= 0) onCommit(next);
  };

  return (
    <EditableRow label={label}>
      <input
        type="number"
        defaultValue={value ?? ""}
        placeholder="none"
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); commit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

function OptionalNumberField({ label, value, onCommit }: {
  label: string;
  value: number | null;
  onCommit: (value: number | null) => void;
}) {
  const [focused, setFocused] = useState(false);

  const commit = (raw: string) => {
    const trimmed = raw.trim();
    if (trimmed === "") {
      onCommit(null);
      return;
    }
    const next = Number(trimmed);
    if (Number.isFinite(next)) onCommit(next);
  };

  return (
    <EditableRow label={label}>
      <input
        type="number"
        defaultValue={value ?? ""}
        placeholder="none"
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); commit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

function ColorField({ label, value, onCommit }: {
  label: string;
  value: [number, number, number, number];
  onCommit: (value: [number, number, number, number]) => void;
}) {
  const [focused, setFocused] = useState<number | null>(null);

  const commit = (idx: number, raw: string) => {
    const next = Number(raw);
    if (!Number.isFinite(next)) return;
    const color: [number, number, number, number] = [...value];
    color[idx] = Math.max(0, Math.min(1, next));
    onCommit(color);
  };

  return (
    <EditableRow label={label}>
      {value.map((channel, idx) => (
        <input
          key={idx}
          type="number"
          step="0.01"
          min="0"
          max="1"
          defaultValue={channel.toFixed(2)}
          onFocus={() => setFocused(idx)}
          onBlur={e => { setFocused(null); commit(idx, e.currentTarget.value); }}
          onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
          style={inputStyle(focused === idx)}
        />
      ))}
    </EditableRow>
  );
}

function EditableRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ display: "flex", alignItems: "center", height: "22px", padding: "0 10px", gap: "6px" }}>
      <span style={{ width: "60px", fontSize: "10px", color: "var(--text-dim)", flexShrink: 0 }}>{label}</span>
      {children}
    </div>
  );
}

function inputStyle(focused: boolean): CSSProperties {
  return {
    flex: 1,
    minWidth: 0,
    height: "18px",
    background: "var(--bg-3)",
    border: `1px solid ${focused ? "var(--accent)" : "var(--border-bright)"}`,
    borderRadius: "var(--radius)",
    color: "var(--text-bright)",
    fontFamily: "var(--font-mono)",
    fontSize: "10px",
    padding: "0 4px",
    outline: "none",
  };
}
