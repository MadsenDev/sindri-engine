import { useState } from "react";
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
        ? <ComponentView entity={entity} component={comp} componentIdx={selectedComponent} onBack={() => onSelectComponent(null)} onSceneChange={onSceneChange} onOpenScript={onOpenScript} />
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
        {component.type === "Sprite"      && <SpriteFields comp={component} />}
        {component.type === "PhysicsBody" && <PhysicsBodyFields comp={component} entityId={entity.id} componentIdx={componentIdx} onSceneChange={onSceneChange} />}
        {component.type === "Collider"    && <ColliderFields comp={component} />}
        {component.type === "Script"      && <ScriptField comp={component} entityId={entity.id} componentIdx={componentIdx} onOpenScript={onOpenScript} onSceneChange={onSceneChange} />}
        {component.type === "Camera"      && <CameraFields comp={component} />}
        {component.type === "AudioSource" && <AudioFields comp={component} />}
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

function SpriteFields({ comp }: { comp: Extract<Component, { type: "Sprite" }> }) {
  return (
    <>
      <Field label="texture" value={comp.texture_path || "(none)"} />
      <Field label="size"    value={`${comp.width} × ${comp.height}`} />
      <Field label="flip"    value={`x:${comp.flip_x}  y:${comp.flip_y}`} />
      <Field label="color"   value={comp.color.map(v => v.toFixed(2)).join(", ")} />
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
  const patch = async (data: Record<string, unknown>) => {
    try {
      await invoke("patch_component", { entityId, componentIdx, data });
      onSceneChange();
    } catch {}
  };

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
      <Field label="damping" value={`lin ${comp.linear_damping} · ang ${comp.angular_damping}`} />
      <Field label="collision" value={`layer ${comp.collision_layer} · mask ${comp.collision_mask}`} />
    </>
  );
}

// ─── Collider ────────────────────────────────────────────────────────────────

function ColliderFields({ comp }: { comp: Extract<Component, { type: "Collider" }> }) {
  return (
    <>
      <Field label="size"    value={`${comp.width} × ${comp.height}`} />
      <Field label="offset"  value={`${comp.offset_x}, ${comp.offset_y}`} />
      <Field label="trigger" value={String(comp.is_trigger)} accent={comp.is_trigger ? "var(--ai)" : undefined} />
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

function CameraFields({ comp }: { comp: Extract<Component, { type: "Camera" }> }) {
  return (
    <>
      <Field label="zoom"   value={comp.zoom.toFixed(2)} />
      <Field label="follow" value={comp.follow_entity !== null ? `entity #${comp.follow_entity}` : "none"} />
    </>
  );
}

// ─── AudioSource ─────────────────────────────────────────────────────────────

function AudioFields({ comp }: { comp: Extract<Component, { type: "AudioSource" }> }) {
  return (
    <>
      <Field label="path"     value={comp.path || "(none)"} />
      <Field label="volume"   value={comp.volume.toFixed(2)} />
      <Field label="loop"     value={String(comp.looping)} />
      <Field label="autoplay" value={String(comp.play_on_start)} />
    </>
  );
}

// ─── Shared ───────────────────────────────────────────────────────────────────

function Field({ label, value, accent }: { label: string; value: string; accent?: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", height: "22px", padding: "0 10px", gap: "6px" }}>
      <span style={{ width: "60px", fontSize: "10px", color: "var(--text-dim)", flexShrink: 0 }}>{label}</span>
      <span style={{ fontSize: "10px", color: accent ?? "var(--text-base)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {value}
      </span>
    </div>
  );
}
