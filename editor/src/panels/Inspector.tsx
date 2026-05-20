import { useState, useEffect, useRef, type CSSProperties, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import type { Entity, Component } from "../App";
import { useContextMenu } from "../components/ContextMenu";

interface AiSuggestion {
  label: string;
  prompt: string;
  mode: "send" | "prefill";
}

function getAiSuggestions(entity: Entity): AiSuggestion[] {
  const types = new Set(entity.components.map(c => c.type));
  const suggestions: AiSuggestion[] = [];

  const add = (label: string, prompt: string, mode: "send" | "prefill") => {
    if (suggestions.length < 3) suggestions.push({ label, prompt, mode });
  };

  if (types.has("Script")) {
    add(`Refactor the ${entity.name} script`, `Refactor the ${entity.name} script to be cleaner and more efficient`, "send");
    add(`Add a new behavior to ${entity.name}`, `Add this new behavior to ${entity.name}: `, "prefill");
  }
  if (types.has("PhysicsBody") && types.has("Collider")) {
    add(`Tune ${entity.name} physics`, `Review and tune the PhysicsBody and Collider settings for ${entity.name}`, "send");
  } else if (types.has("PhysicsBody") && !types.has("Collider")) {
    add(`Add a collider to ${entity.name}`, `Add a Collider component to ${entity.name} matching its sprite size`, "send");
  } else if (!types.has("PhysicsBody") && !types.has("Script")) {
    add(`Add physics to ${entity.name}`, `Add physics (PhysicsBody + Collider) to ${entity.name}: `, "prefill");
  }
  if (types.has("Camera")) {
    add(`Configure ${entity.name} follow`, `Set up the ${entity.name} camera to smoothly follow the player`, "send");
  }
  if (types.has("Sprite") && !types.has("Script")) {
    add(`Animate ${entity.name} with a script`, `Add a Lua script to ${entity.name} that animates it. What should it do? `, "prefill");
  }
  if (types.has("AudioSource")) {
    add(`Trigger ${entity.name} audio on event`, `Set up ${entity.name} so its audio triggers on: `, "prefill");
  }
  if (!types.has("Sprite") && !types.has("Camera")) {
    add(`Add a sprite to ${entity.name}`, `Add a Sprite component to ${entity.name}`, "send");
  }

  const fallbacks: AiSuggestion[] = [
    { label: `Explain ${entity.name}'s purpose`, prompt: `Explain what ${entity.name} does in this scene`, mode: "send" },
    { label: `Add a behavior to ${entity.name}`, prompt: `Add this behavior to ${entity.name}: `, mode: "prefill" },
    { label: `Optimize ${entity.name}`, prompt: `Suggest optimizations for ${entity.name}`, mode: "send" },
  ];
  for (const f of fallbacks) {
    if (suggestions.length >= 3) break;
    suggestions.push(f);
  }

  return suggestions.slice(0, 3);
}

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
  selectedComponent: number | null;
  onSelectComponent: (idx: number | null) => void;
  onSceneChange: () => void;
  onOpenScript: (path: string) => void;
  onAskAI?: (prompt: string, mode: "send" | "prefill") => void;
  suggestionModel?: string | null;
}

export default function Inspector({ entity, selectedComponent, onSelectComponent, onSceneChange, onOpenScript, onAskAI, suggestionModel }: Props) {
  const [aiSuggestions, setAiSuggestions] = useState<AiSuggestion[] | null>(null);
  const [suggestionsLoading, setSuggestionsLoading] = useState(false);
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);
  const [regenerateKey, setRegenerateKey] = useState(0);
  const lastEntityRef = useRef<string | null>(null);
  const contextMenu = useContextMenu();

  const regenerate = () => {
    lastEntityRef.current = null;
    setAiSuggestions(null);
    setRegenerateKey(k => k + 1);
  };

  useEffect(() => {
    if (!entity || !suggestionModel) { setAiSuggestions(null); return; }

    const cacheKey = `${entity.id}:${entity.components.map(c => c.type).join(",")}`;
    if (cacheKey === lastEntityRef.current) return;
    lastEntityRef.current = cacheKey;

    setAiSuggestions(null);
    setSuggestionsLoading(true);

    invoke<{ label: string; prompt: string; mode: string }[]>(
      "generate_entity_suggestions",
      { entityName: entity.name, components: entity.components, model: suggestionModel }
    )
      .then(results => {
        const valid = results
          .filter(r => r.label && r.prompt)
          .map(r => ({ label: r.label, prompt: r.prompt, mode: (r.mode === "prefill" ? "prefill" : "send") as "send" | "prefill" }));
        if (valid.length > 0) setAiSuggestions(valid.slice(0, 3));
      })
      .catch(() => { /* fall back to rules-based */ })
      .finally(() => setSuggestionsLoading(false));
  }, [entity?.id, entity?.components.length, suggestionModel, regenerateKey]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!entity) {
    return (
      <div style={{
        flex: 1, display: "flex", flexDirection: "column",
        overflow: "hidden", fontFamily: "var(--font-ui)",
      }}>
        <EmptyInspector onAskAI={onAskAI} />
      </div>
    );
  }

  const comp = selectedComponent !== null ? entity.components[selectedComponent] ?? null : null;

  return (
    <div style={{
      flex: 1, display: "flex", flexDirection: "column",
      overflow: "hidden", fontFamily: "var(--font-ui)",
    }}>
      {/* Entity header */}
      <div style={{
        padding: "22px 22px 18px",
        borderBottom: "1px solid var(--rule)",
        flexShrink: 0,
      }}>
        <div style={{
          fontSize: "11.5px", color: "var(--ink-3)",
          display: "flex", alignItems: "center", gap: "8px",
        }}>
          <span style={{
            width: "8px", height: "8px",
            background: entity.active ? "var(--moss)" : "var(--ink-4)",
            display: "inline-block",
          }} />
          <span>entity</span>
        </div>
        <div style={{
          fontFamily: "var(--font-ui)", fontSize: "32px",
          lineHeight: 1.05, marginTop: "4px", color: "var(--ink)",
          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
        }}>
          {entity.name}
        </div>
        <div style={{
          marginTop: "10px", fontFamily: "var(--font-mono)",
          fontSize: "11px", color: "var(--ink-4)",
        }}>
          #{entity.id} · {entity.components.length} component{entity.components.length !== 1 ? "s" : ""}
        </div>
      </div>

      {/* Scrollable body */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {comp !== null && selectedComponent !== null ? (
          <ComponentView
            key={`${entity.id}:${selectedComponent}:${comp.type}`}
            entity={entity}
            component={comp}
            componentIdx={selectedComponent}
            onBack={() => onSelectComponent(null)}
            onSceneChange={onSceneChange}
            onOpenScript={onOpenScript}
          />
        ) : (
          <>
            {/* Component list */}
            {entity.components.length === 0 ? (
              <div style={{ padding: "16px 22px", color: "var(--ink-3)", fontSize: "12.5px" }}>
                No components. Right-click entity to add one.
              </div>
            ) : (
              entity.components.map((c, idx) => (
                <div key={`${c.type}-${idx}`} style={{ borderBottom: "1px solid var(--rule)" }}>
                  <button
                    onClick={() => onSelectComponent(idx)}
                    style={{
                      display: "flex", alignItems: "center", gap: "8px",
                      width: "100%", padding: "14px 22px",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      textAlign: "left",
                    }}
                    onMouseEnter={e => (e.currentTarget as HTMLElement).style.background = "var(--paper-3)"}
                    onMouseLeave={e => (e.currentTarget as HTMLElement).style.background = "none"}
                  >
                    <span style={{ fontSize: "12px", color: "var(--ink-3)", width: "16px", textAlign: "center", flexShrink: 0 }}>
                      {COMPONENT_ICON[c.type] ?? "·"}
                    </span>
                    <span style={{
                      fontFamily: "var(--font-ui)", fontSize: "12.5px",
                      color: "var(--ink)", flex: 1,
                    }}>
                      {c.type === "Script" && (c as Extract<Component, { type: "Script" }>).path
                        ? `Script · ${(c as Extract<Component, { type: "Script" }>).path.split("/").pop()}`
                        : c.type}
                    </span>
                    <span style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "14px" }}>···</span>
                  </button>
                </div>
              ))
            )}

            {/* Add component */}
            <div style={{ margin: "8px 22px 14px" }}>
              <button style={{
                width: "100%", padding: "10px 12px",
                fontSize: "12.5px", color: "var(--ink-3)",
                border: "1px dashed var(--rule-2)",
                background: "none", cursor: "pointer",
                display: "flex", alignItems: "center", gap: "8px",
                fontFamily: "var(--font-ui)",
              }}>
                + Add component
              </button>
            </div>

            {/* AI suggestions */}
            {(() => {
              const suggestions = aiSuggestions ?? getAiSuggestions(entity);
              const previewed = hoveredIdx !== null ? suggestions[hoveredIdx] : null;
              return (
                <div style={{ padding: "16px 22px", borderTop: "1px solid var(--rule)" }}>
                  <span style={{
                    fontSize: "11.5px", color: "var(--amber)",
                    display: "inline-flex", alignItems: "center", gap: "6px",
                  }}>
                    ✦ Ask about <span style={{ color: "var(--ink)" }}>{entity.name}</span>
                    {suggestionsLoading && (
                      <span style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px" }}>…</span>
                    )}
                  </span>

                  <div style={{ marginTop: "10px", display: "flex", flexDirection: "column" }}>
                    {suggestions.map((s, i) => (
                      <button
                        key={i}
                        onClick={() => onAskAI?.(s.prompt, s.mode)}
                        onContextMenu={e => {
                          e.preventDefault();
                          contextMenu.show(e.clientX, e.clientY, [
                            {
                              label: "Send now",
                              icon: "→",
                              onClick: () => onAskAI?.(s.prompt, "send"),
                            },
                            {
                              label: "Modify before sending",
                              icon: "✎",
                              onClick: () => onAskAI?.(s.prompt, "prefill"),
                            },
                            { divider: true },
                            {
                              label: "Regenerate suggestions",
                              icon: "↻",
                              disabled: !suggestionModel,
                              onClick: regenerate,
                            },
                          ]);
                        }}
                        onMouseEnter={e => {
                          (e.currentTarget as HTMLElement).style.color = "var(--ink)";
                          setHoveredIdx(i);
                        }}
                        onMouseLeave={e => {
                          (e.currentTarget as HTMLElement).style.color = "var(--ink-2)";
                          setHoveredIdx(null);
                        }}
                        style={{
                          display: "flex", alignItems: "center", gap: "8px",
                          fontSize: "13px", color: "var(--ink-2)",
                          padding: "6px 0",
                          background: "none", border: "none",
                          borderBottom: i < suggestions.length - 1 ? "1px dotted var(--paper-3)" : "none",
                          cursor: "pointer", textAlign: "left",
                          width: "100%",
                          fontFamily: "var(--font-ui)",
                        }}
                      >
                        <span style={{ flex: 1 }}>{s.label}</span>
                        <span style={{
                          color: "var(--ink-4)", fontSize: "10px",
                          fontFamily: "var(--font-mono)",
                          border: "1px solid var(--rule-2)", padding: "1px 4px",
                          flexShrink: 0,
                        }}>
                          {s.mode === "prefill" ? "fill" : "→"}
                        </span>
                      </button>
                    ))}
                  </div>

                  {/* Prompt preview / hint */}
                  <div style={{
                    marginTop: "8px",
                    paddingTop: "6px",
                    borderTop: "1px dotted var(--rule)",
                    minHeight: "32px",
                    fontFamily: "var(--font-mono)", fontSize: "10.5px",
                    color: "var(--ink-4)", lineHeight: 1.5,
                    wordBreak: "break-word",
                  }}>
                    {previewed
                      ? previewed.prompt
                      : <span style={{ opacity: 0.45 }}>right-click for options</span>
                    }
                  </div>
                </div>
              );
            })()}
          </>
        )}
      </div>
    </div>
  );
}

function EmptyInspector({ onAskAI }: { onAskAI?: (prompt: string, mode: "send" | "prefill") => void }) {
  return (
    <div style={{ padding: "28px 22px", display: "flex", flexDirection: "column", gap: "14px" }}>
      <div style={{
        fontFamily: "var(--font-ui)", fontSize: "26px",
        lineHeight: 1.2, color: "var(--ink-2)",
      }}>
        Nothing selected.
      </div>
      <div style={{ fontSize: "12.5px", color: "var(--ink-3)", lineHeight: 1.5 }}>
        Pick an entity in the scene to inspect it — or press{" "}
        <span style={{ fontFamily: "var(--font-mono)", color: "var(--ink-4)", border: "1px solid var(--rule-2)", padding: "1px 4px" }}>Ctrl</span>
        {" "}<span style={{ fontFamily: "var(--font-mono)", color: "var(--ink-4)", border: "1px solid var(--rule-2)", padding: "1px 4px" }}>K</span>
        {" "}to ask the assistant.
      </div>
      {onAskAI && (
        <button
          onClick={() => onAskAI("What should I add to this scene?", "send")}
          style={{
            marginTop: "8px",
            display: "inline-flex", alignItems: "center", gap: "8px",
            fontSize: "13px", color: "var(--amber)",
            background: "none", border: "none", cursor: "pointer",
            fontFamily: "var(--font-ui)", padding: 0,
          }}
        >
          ✦ Ask Sindri for ideas
        </button>
      )}
    </div>
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
      <div style={{
        display: "flex", alignItems: "center", gap: "0",
        height: "40px", borderBottom: "1px solid var(--rule)", flexShrink: 0,
        padding: "0 22px",
      }}>
        <button
          onClick={onBack}
          style={{
            background: "none", border: "none", color: "var(--ink-3)",
            cursor: "pointer", padding: "0 8px 0 0",
            fontFamily: "var(--font-mono)", fontSize: "14px",
            display: "flex", alignItems: "center",
          }}
        >
          ‹
        </button>
        <span style={{ fontSize: "11px", color: "var(--ink-3)", fontFamily: "var(--font-ui)" }}>{entity.name}</span>
        <span style={{ fontSize: "12px", color: "var(--ink-4)", margin: "0 6px" }}>›</span>
        <span style={{ fontSize: "11px", color: "var(--ink-4)" }}>{icon}</span>
        <span style={{
          fontFamily: "var(--font-ui)", fontSize: "12px",
          color: "var(--ink)", marginLeft: "6px",
        }}>{component.type}</span>
      </div>

      <div style={{ padding: "8px 0 4px" }}>
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
    try { await invoke("patch_transform", args); onSceneChange(); } catch {}
  };

  const rows: { label: string; fields: { key: string; value: number }[] }[] = [
    { label: "position", fields: [{ key: "x", value: comp.x }, { key: "y", value: comp.y }] },
    { label: "scale",    fields: [{ key: "scale_x", value: comp.scale_x }, { key: "scale_y", value: comp.scale_y }] },
    { label: "rotation", fields: [{ key: "rotation", value: comp.rotation }] },
  ];

  return (
    <>
      {rows.map(row => (
        <div key={row.label} style={{ display: "flex", alignItems: "center", height: "28px", padding: "0 22px", gap: "8px" }}>
          <span style={{ width: "78px", fontSize: "12px", color: "var(--ink-3)", flexShrink: 0 }}>{row.label}</span>
          {row.fields.map(f => (
            <input
              key={f.key}
              defaultValue={f.value.toFixed(2)}
              onFocus={() => setFocused(f.key)}
              onBlur={e => { setFocused(null); patch(f.key, e.target.value); }}
              onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
              style={inputStyle(focused === f.key)}
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
  entityId: number; componentIdx: number; onSceneChange: () => void;
}) {
  const patch = useComponentPatch(entityId, componentIdx, onSceneChange);
  const browseTexture = async () => {
    const file = await openFileDialog({
      multiple: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "bmp", "webp"] }],
    });
    if (file) patch({ texture_path: file as string });
  };
  return (
    <>
      <BrowseInputField label="texture" value={comp.texture_path} placeholder="(none)" onCommit={v => patch({ texture_path: v })} onBrowse={browseTexture} />
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
  entityId: number; componentIdx: number; onSceneChange: () => void;
}) {
  const patch = useComponentPatch(entityId, componentIdx, onSceneChange);
  return (
    <>
      <EditableRow label="type">
        <CustomSelect
          value={comp.body_type}
          options={["Dynamic", "Kinematic", "Fixed"]}
          onChange={v => patch({ body_type: v })}
        />
      </EditableRow>
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
  entityId: number; componentIdx: number; onSceneChange: () => void;
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

// ─── Script ──────────────────────────────────────────────────────────────────

function ScriptField({ comp, entityId, componentIdx, onOpenScript, onSceneChange }: {
  comp: Extract<Component, { type: "Script" }>;
  entityId: number; componentIdx: number;
  onOpenScript: (path: string) => void; onSceneChange: () => void;
}) {
  const [focused, setFocused] = useState(false);
  const patchPath = async (newPath: string) => {
    try { await invoke("patch_component", { entityId, componentIdx, data: { path: newPath } }); onSceneChange(); } catch {}
  };
  return (
    <div style={{ padding: "2px 22px 8px" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "8px", height: "28px" }}>
        <span style={{ width: "78px", fontSize: "12px", color: "var(--ink-3)", flexShrink: 0 }}>path</span>
        <input
          defaultValue={comp.path || ""}
          placeholder="(none)"
          onFocus={() => setFocused(true)}
          onBlur={e => { setFocused(false); patchPath(e.target.value); }}
          onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
          style={inputStyle(focused)}
        />
      </div>
      <button
        onClick={() => comp.path && onOpenScript(comp.path)}
        disabled={!comp.path}
        style={{
          marginTop: "6px",
          background: "var(--paper-2)", border: "1px solid var(--rule-2)",
          color: comp.path ? "var(--cyan)" : "var(--ink-4)",
          fontFamily: "var(--font-ui)", fontSize: "12px",
          padding: "5px 12px", cursor: comp.path ? "pointer" : "default", width: "100%",
        }}
      >Open in editor</button>
    </div>
  );
}

// ─── Camera ──────────────────────────────────────────────────────────────────

function CameraFields({ comp, entityId, componentIdx, onSceneChange }: {
  comp: Extract<Component, { type: "Camera" }>;
  entityId: number; componentIdx: number; onSceneChange: () => void;
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
  entityId: number; componentIdx: number; onSceneChange: () => void;
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

// ─── Shared field components ──────────────────────────────────────────────────

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
  label: string; value: string; placeholder?: string; onCommit: (value: string) => void;
}) {
  const [focused, setFocused] = useState(false);
  return (
    <EditableRow label={label}>
      <input
        defaultValue={value} placeholder={placeholder}
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); onCommit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

function NumberInputField({ label, value, decimals = 2, min, max, onCommit }: {
  label: string; value: number; decimals?: number; min?: number; max?: number; onCommit: (value: number) => void;
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
  label: string; value: number | null; onCommit: (value: number | null) => void;
}) {
  const [focused, setFocused] = useState(false);
  const commit = (raw: string) => {
    const trimmed = raw.trim();
    if (trimmed === "") { onCommit(null); return; }
    const next = Number(trimmed);
    if (Number.isInteger(next) && next >= 0) onCommit(next);
  };
  return (
    <EditableRow label={label}>
      <input
        type="number" defaultValue={value ?? ""} placeholder="none"
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); commit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

function OptionalNumberField({ label, value, onCommit }: {
  label: string; value: number | null; onCommit: (value: number | null) => void;
}) {
  const [focused, setFocused] = useState(false);
  const commit = (raw: string) => {
    const trimmed = raw.trim();
    if (trimmed === "") { onCommit(null); return; }
    const next = Number(trimmed);
    if (Number.isFinite(next)) onCommit(next);
  };
  return (
    <EditableRow label={label}>
      <input
        type="number" defaultValue={value ?? ""} placeholder="none"
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); commit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

function BoolField({ label, value, onChange }: { label: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <label style={{ display: "flex", alignItems: "center", height: "24px", padding: "0 22px", gap: "8px", cursor: "pointer" }}>
      <span style={{ width: "78px", fontSize: "12px", color: "var(--ink-3)", flexShrink: 0 }}>{label}</span>
      <input type="checkbox" checked={value} onChange={e => onChange(e.currentTarget.checked)} />
      <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: value ? "var(--amber)" : "var(--ink-4)" }}>
        {String(value)}
      </span>
    </label>
  );
}

function ColorField({ label, value, onCommit }: {
  label: string; value: [number, number, number, number]; onCommit: (value: [number, number, number, number]) => void;
}) {
  const [focused, setFocused] = useState<number | null>(null);
  const pickerRef = useRef<HTMLInputElement>(null);

  const toHex = (c: [number, number, number, number]) =>
    "#" + [c[0], c[1], c[2]].map(v => Math.round(v * 255).toString(16).padStart(2, "0")).join("");

  const commit = (idx: number, raw: string) => {
    const next = Number(raw);
    if (!Number.isFinite(next)) return;
    const color: [number, number, number, number] = [...value] as [number, number, number, number];
    color[idx] = Math.max(0, Math.min(1, next));
    onCommit(color);
  };

  const handlePickerChange = (hex: string) => {
    const r = parseInt(hex.slice(1, 3), 16) / 255;
    const g = parseInt(hex.slice(3, 5), 16) / 255;
    const b = parseInt(hex.slice(5, 7), 16) / 255;
    onCommit([r, g, b, value[3]]);
  };

  const swatchBg = `rgba(${Math.round(value[0]*255)},${Math.round(value[1]*255)},${Math.round(value[2]*255)},${value[3]})`;

  return (
    <EditableRow label={label}>
      <div
        onClick={() => pickerRef.current?.click()}
        title="Open color picker"
        style={{
          width: "20px", height: "20px", flexShrink: 0,
          background: swatchBg, border: "1px solid var(--rule-2)",
          cursor: "pointer",
        }}
      />
      <input
        ref={pickerRef}
        type="color"
        value={toHex(value)}
        onChange={e => handlePickerChange(e.target.value)}
        style={{ position: "absolute", opacity: 0, pointerEvents: "none", width: 0, height: 0 }}
      />
      {value.map((channel, idx) => (
        <input
          key={idx} type="number" step="0.01" min="0" max="1"
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

function BrowseInputField({ label, value, placeholder, onCommit, onBrowse }: {
  label: string; value: string; placeholder?: string; onCommit: (value: string) => void; onBrowse: () => void;
}) {
  const [focused, setFocused] = useState(false);
  return (
    <EditableRow label={label}>
      <input
        defaultValue={value} placeholder={placeholder}
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); onCommit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={{ ...inputStyle(focused), flex: 1 }}
      />
      <button
        onClick={onBrowse}
        title="Browse"
        style={{
          flexShrink: 0, height: "20px", padding: "0 6px",
          background: "var(--paper-2)", border: "1px solid var(--rule-2)",
          color: "var(--ink-3)", fontFamily: "var(--font-ui)", fontSize: "11px",
          cursor: "pointer",
        }}
        onMouseEnter={e => (e.currentTarget as HTMLElement).style.color = "var(--ink)"}
        onMouseLeave={e => (e.currentTarget as HTMLElement).style.color = "var(--ink-3)"}
      >
        ···
      </button>
    </EditableRow>
  );
}

function CustomSelect({ value, options, onChange }: {
  value: string; options: string[]; onChange: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", handler);
    return () => window.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div ref={ref} style={{ flex: 1, position: "relative" }}>
      <button
        onClick={() => setOpen(o => !o)}
        style={{
          width: "100%", height: "20px",
          background: "var(--paper-2)", border: "1px solid var(--rule-2)",
          color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px",
          padding: "0 6px", cursor: "pointer",
          display: "flex", alignItems: "center", justifyContent: "space-between",
        }}
      >
        <span>{value}</span>
        <span style={{ color: "var(--ink-4)", fontSize: "9px", marginLeft: "4px" }}>▾</span>
      </button>
      {open && (
        <div style={{
          position: "absolute", top: "100%", left: 0, right: 0, zIndex: 200,
          background: "var(--paper)", border: "1px solid var(--rule-2)",
          boxShadow: "0 4px 16px rgba(0,0,0,0.4)",
        }}>
          {options.map(opt => (
            <button
              key={opt}
              onClick={() => { onChange(opt); setOpen(false); }}
              style={{
                display: "block", width: "100%", padding: "6px 8px",
                background: opt === value ? "var(--paper-3)" : "none",
                border: "none", color: "var(--ink)",
                fontFamily: "var(--font-mono)", fontSize: "11px",
                cursor: "pointer", textAlign: "left",
              }}
              onMouseEnter={e => (e.currentTarget as HTMLElement).style.background = "var(--paper-3)"}
              onMouseLeave={e => (e.currentTarget as HTMLElement).style.background = opt === value ? "var(--paper-3)" : "none"}
            >
              {opt}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function EditableRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ display: "flex", alignItems: "center", height: "26px", padding: "0 22px", gap: "8px" }}>
      <span style={{ width: "78px", fontSize: "12px", color: "var(--ink-3)", flexShrink: 0 }}>{label}</span>
      {children}
    </div>
  );
}

function inputStyle(focused: boolean): CSSProperties {
  return {
    flex: 1, minWidth: 0,
    height: "20px",
    background: "var(--paper-2)",
    border: `1px solid ${focused ? "var(--amber)" : "var(--rule-2)"}`,
    color: "var(--ink)",
    fontFamily: "var(--font-mono)",
    fontSize: "12px",
    padding: "0 6px",
    outline: "none",
  };
}
