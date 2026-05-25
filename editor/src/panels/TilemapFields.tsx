import { useState, useEffect, useRef, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TilemapEdit } from "../App";
import type { Component } from "../App";
import { useComponentPatch, NumberInputField, ColorField, BrowseInputField } from "../components/InspectorFields";
import FilePicker, { type ProjectFile } from "../components/FilePicker";

function resolveTextureUrl(path: string): string {
  if (!path || path.startsWith("http") || path.startsWith("data:")) return path;
  return `http://localhost:7878/assets/${path}`;
}

interface PrefabInfo { name: string; path: string; }

export default function TilemapFields({ comp, entityId, componentIdx, onSceneChange, projectFiles, projectPath, tilemapEdit, onTilemapEditChange, onOpenPalette }: {
  comp: Extract<Component, { type: "Tilemap" }>;
  entityId: number; componentIdx: number; onSceneChange: () => void;
  projectFiles: ProjectFile[];
  projectPath?: string | null;
  entityX?: number;
  entityY?: number;
  tilemapEdit: TilemapEdit;
  onTilemapEditChange: (e: TilemapEdit) => void;
  onOpenPalette?: (path: string) => void;
}) {
  const patch = useComponentPatch(entityId, componentIdx, onSceneChange);
  const [pickingPaletteTex, setPickingPaletteTex] = useState<number | null>(null);
  const [pickingPaletteFile, setPickingPaletteFile] = useState(false);
  const [paletteImages, setPaletteImages] = useState<Map<string, HTMLImageElement>>(new Map());
  const [prefabs, setPrefabs] = useState<PrefabInfo[]>([]);
  const [editingLayerName, setEditingLayerName] = useState<number | null>(null);
  const [palettesOpen, setPalettesOpen] = useState(false);
  const loadedPathsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    comp.palettes.forEach(p => {
      if (!p.texture_path || loadedPathsRef.current.has(p.texture_path)) return;
      loadedPathsRef.current.add(p.texture_path);
      const img = new Image();
      img.onload = () => setPaletteImages(prev => new Map(prev).set(p.texture_path, img));
      img.src = resolveTextureUrl(p.texture_path);
    });
  }, [comp.palettes]);

  useEffect(() => {
    if (tilemapEdit.mode !== "stamp" || !projectPath) return;
    invoke<PrefabInfo[]>("list_prefabs", { projectPath }).then(setPrefabs).catch(() => {});
  }, [tilemapEdit.mode, projectPath]);

  const set = (partial: Partial<TilemapEdit>) => onTilemapEditChange({ ...tilemapEdit, ...partial });

  const addPalette = () => {
    const newPalette = { name: `Palette ${comp.palettes.length + 1}`, texture_path: "", tileset_cols: 4, tileset_rows: 4, margin: 0, spacing: 0, solid_tiles: [] };
    patch({ palettes: [...comp.palettes, newPalette] });
  };

  const loadPaletteFromFile = async (relativePath: string) => {
    if (!projectPath) return;
    try {
      const data = await invoke<{ name: string; texture_path: string; tileset_cols: number; tileset_rows: number; margin?: number; spacing?: number; solid_tiles?: number[] }>(
        "read_tile_palette", { projectPath, relativePath }
      );
      const newPalette = {
        name: data.name || relativePath.split("/").pop()?.replace(/\.tilepal$/, "") || "palette",
        texture_path: data.texture_path || "",
        tileset_cols: data.tileset_cols || 4,
        tileset_rows: data.tileset_rows || 4,
        margin: data.margin ?? 0,
        spacing: data.spacing ?? 0,
        solid_tiles: data.solid_tiles ?? [],
      };
      patch({ palettes: [...comp.palettes, newPalette] });
    } catch (e) {
      console.error("read_tile_palette failed:", e);
    }
  };

  const removePalette = (i: number) => {
    patch({ palettes: comp.palettes.filter((_, idx) => idx !== i) });
  };

  const patchPalette = (i: number, fields: Partial<typeof comp.palettes[0]>) => {
    patch({ palettes: comp.palettes.map((p, idx) => idx === i ? { ...p, ...fields } : p) });
  };

  const toggleSolidTile = (tileIdx: number) => {
    const pal = comp.palettes[tilemapEdit.paletteIdx];
    if (!pal) return;
    const solid = pal.solid_tiles.includes(tileIdx)
      ? pal.solid_tiles.filter(id => id !== tileIdx)
      : [...pal.solid_tiles, tileIdx];
    patchPalette(tilemapEdit.paletteIdx, { solid_tiles: solid });
  };

  const addLayer = () => {
    const tileCount = comp.map_cols * comp.map_rows;
    const newLayer = { name: `Layer ${comp.layers.length + 1}`, tiles: new Array(tileCount).fill(0), visible: true, opacity: 1.0, z_index: 0 };
    const next = [...comp.layers, newLayer];
    patch({ layers: next });
    set({ layerIdx: next.length - 1 });
  };

  const removeLayer = (i: number) => {
    if (comp.layers.length <= 1) return;
    const next = comp.layers.filter((_, idx) => idx !== i);
    patch({ layers: next });
    set({ layerIdx: Math.min(tilemapEdit.layerIdx, next.length - 1) });
  };

  const toggleLayerVisible = (i: number) => {
    patch({ layers: comp.layers.map((l, idx) => idx === i ? { ...l, visible: !l.visible } : l) });
  };

  const renameLayer = (i: number, name: string) => {
    patch({ layers: comp.layers.map((l, idx) => idx === i ? { ...l, name } : l) });
    setEditingLayerName(null);
  };

  const moveLayer = (i: number, dir: -1 | 1) => {
    const j = i + dir;
    if (j < 0 || j >= comp.layers.length) return;
    const next = [...comp.layers];
    [next[i], next[j]] = [next[j], next[i]];
    patch({ layers: next });
    set({ layerIdx: j });
  };

  const pal = comp.palettes[tilemapEdit.paletteIdx];
  const palImg = pal ? paletteImages.get(pal.texture_path) : undefined;
  const tsC = Math.max(1, pal?.tileset_cols ?? 1);
  const tsR = Math.max(1, pal?.tileset_rows ?? 1);
  const CELL = Math.max(16, Math.min(40, Math.floor(220 / tsC)));

  const modeBtn = (mode: TilemapEdit["mode"], label: string) => (
    <button
      key={mode}
      onClick={() => set({ mode })}
      style={{
        flex: 1, padding: "4px 0", fontFamily: "var(--font-mono)", fontSize: "10px", cursor: "pointer",
        background: tilemapEdit.mode === mode
          ? (mode === "erase" ? "rgba(200,60,60,0.8)" : mode === "collision" ? "rgba(200,60,60,0.8)" : mode === "stamp" ? "rgba(160,120,40,0.8)" : "var(--amber)")
          : "none",
        color: tilemapEdit.mode === mode ? "var(--paper)" : "var(--ink-3)",
        border: `1px solid ${tilemapEdit.mode === mode ? "transparent" : "var(--rule-2)"}`,
      }}
    >{label}</button>
  );

  return (
    <>
      {pickingPaletteTex !== null && (
        <FilePicker title="Pick tileset texture" kinds={["image"]} files={projectFiles}
          onSelect={p => { patchPalette(pickingPaletteTex!, { texture_path: p }); setPickingPaletteTex(null); }}
          onClose={() => setPickingPaletteTex(null)} />
      )}
      {pickingPaletteFile && (
        <FilePicker title="Load .tilepal" kinds={["tilepal"]} files={projectFiles}
          onSelect={p => { loadPaletteFromFile(p); setPickingPaletteFile(false); }}
          onClose={() => setPickingPaletteFile(false)} />
      )}

      <NumberInputField label="tile w" value={comp.tile_width} onCommit={v => patch({ tile_width: v })} />
      <NumberInputField label="tile h" value={comp.tile_height} onCommit={v => patch({ tile_height: v })} />
      <NumberInputField label="map cols" value={comp.map_cols} decimals={0} min={1} onCommit={v => patch({ map_cols: Math.round(v) })} />
      <NumberInputField label="map rows" value={comp.map_rows} decimals={0} min={1} onCommit={v => patch({ map_rows: Math.round(v) })} />
      <ColorField label="tint" value={comp.tint} onCommit={v => patch({ tint: v })} />

      {/* Mode buttons */}
      <div style={{ display: "flex", gap: "3px", padding: "8px 12px 6px", borderTop: "1px solid var(--rule)" }}>
        {modeBtn("draw", "Draw")}
        {modeBtn("erase", "Erase")}
        {modeBtn("collision", "Collision")}
        {projectPath && modeBtn("stamp", "Stamp")}
      </div>

      {/* Hint */}
      <div style={{ padding: "3px 12px 6px" }}>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)" }}>
          {tilemapEdit.mode === "draw" && "Click/drag in scene to paint"}
          {tilemapEdit.mode === "erase" && "Click/drag in scene to erase tiles"}
          {tilemapEdit.mode === "collision" && "Click tiles below to toggle solid · Solid tiles shown in viewport"}
          {tilemapEdit.mode === "stamp" && "Select a prefab below · click scene to place"}
        </span>
      </div>

      {/* Stamp mode: prefab list */}
      {tilemapEdit.mode === "stamp" && (
        <div style={{ borderTop: "1px solid var(--rule)", padding: "6px 0" }}>
          <div style={{ padding: "3px 12px 5px" }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)" }}>PREFABS</span>
          </div>
          {prefabs.length === 0 ? (
            <div style={{ padding: "4px 12px", fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>No prefabs found</div>
          ) : prefabs.map(p => (
            <div
              key={p.path}
              onClick={() => set({ selectedPrefabPath: tilemapEdit.selectedPrefabPath === p.path ? null : p.path })}
              style={{
                display: "flex", alignItems: "center", gap: "7px",
                padding: "4px 12px", cursor: "pointer",
                background: tilemapEdit.selectedPrefabPath === p.path ? "rgba(160,120,40,0.25)" : "none",
                borderLeft: tilemapEdit.selectedPrefabPath === p.path ? "2px solid var(--amber)" : "2px solid transparent",
              }}
            >
              <span style={{ fontSize: "9px", color: "var(--amber)" }}>◆</span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink)" }}>{p.name}</span>
            </div>
          ))}
        </div>
      )}

      {/* Non-stamp mode: palette tabs + tile picker */}
      {tilemapEdit.mode !== "stamp" && (
        <div style={{ borderTop: "1px solid var(--rule)" }}>
          {comp.palettes.length > 1 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: "2px", padding: "5px 8px 4px" }}>
              {comp.palettes.map((p, i) => (
                <button key={i} onClick={() => set({ paletteIdx: i, tileIdx: 0 })} style={{
                  fontFamily: "var(--font-mono)", fontSize: "9px", padding: "2px 6px", cursor: "pointer",
                  background: i === tilemapEdit.paletteIdx ? "var(--amber)" : "none",
                  border: `1px solid ${i === tilemapEdit.paletteIdx ? "transparent" : "var(--rule-2)"}`,
                  color: i === tilemapEdit.paletteIdx ? "var(--paper)" : "var(--ink-3)",
                }}>{p.name || `P${i + 1}`}</button>
              ))}
            </div>
          )}

          <div style={{ padding: "4px 8px 6px", overflowX: "auto" }}>
            {pal && palImg ? (
              <div style={{
                display: "grid",
                gridTemplateColumns: `repeat(${tsC}, ${CELL}px)`,
                width: `${tsC * CELL}px`,
              }}>
                {Array.from({ length: tsC * tsR }, (_, i) => {
                  const isDisabled = pal.disabled_tiles?.includes(i);
                  if (isDisabled && tilemapEdit.mode !== "collision") return null;
                  const m = pal.margin ?? 0;
                  const s = pal.spacing ?? 0;
                  const iw = palImg.naturalWidth;
                  const ih = palImg.naturalHeight;
                  const bgW = CELL * tsC;
                  const bgH = CELL * tsR;
                  let bgX: number, bgY: number;
                  if (m === 0 && s === 0) {
                    bgX = -(i % tsC) * CELL;
                    bgY = -Math.floor(i / tsC) * CELL;
                  } else {
                    const cw = (iw - 2 * m - s * (tsC - 1)) / tsC;
                    const ch = (ih - 2 * m - s * (tsR - 1)) / tsR;
                    bgX = -(m + (i % tsC) * (cw + s)) / iw * bgW;
                    bgY = -(m + Math.floor(i / tsC) * (ch + s)) / ih * bgH;
                  }
                  const isSelected = tilemapEdit.tileIdx === i && tilemapEdit.mode !== "collision";
                  const isSolid = pal.solid_tiles.includes(i) || (pal.tile_colliders?.[i] && pal.tile_colliders[i].type !== "none");
                  return (
                    <div
                      key={i}
                      title={`Tile #${i}${isSolid ? " (solid)" : ""}${isDisabled ? " (hidden)" : ""}`}
                      onClick={() => tilemapEdit.mode === "collision" ? toggleSolidTile(i) : set({ tileIdx: i, ...(tilemapEdit.mode === "erase" ? { mode: "draw" } : {}) })}
                      style={{
                        width: CELL, height: CELL, position: "relative", cursor: "pointer", boxSizing: "border-box",
                        backgroundImage: `url(${resolveTextureUrl(pal.texture_path)})`,
                        backgroundPosition: `${bgX}px ${bgY}px`,
                        backgroundSize: `${bgW}px ${bgH}px`,
                        outline: isSelected ? "2px solid var(--amber)" : isSolid ? "2px solid rgba(220,80,80,0.8)" : "1px solid rgba(255,255,255,0.05)",
                        outlineOffset: "-2px",
                        imageRendering: "pixelated",
                        opacity: isDisabled ? 0.4 : 1,
                      }}
                    >
                      {isSolid && <div style={{ position: "absolute", inset: 0, background: "rgba(220,60,60,0.25)", pointerEvents: "none" }} />}
                    </div>
                  );
                })}
              </div>
            ) : (
              <div style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "6px 0" }}>
                {comp.palettes.length === 0 ? "Add a palette below" : "Loading…"}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Layer list */}
      <div style={{ borderTop: "1px solid var(--rule)" }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "5px 12px" }}>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)" }}>LAYERS</span>
          <button onClick={addLayer} style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "9px", padding: "1px 6px", cursor: "pointer" }}>+ Add</button>
        </div>
        {[...comp.layers].reverse().map((layer, ri) => {
          const i = comp.layers.length - 1 - ri;
          const isActive = i === tilemapEdit.layerIdx;
          const iconBtn: CSSProperties = { background: "none", border: "none", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px", cursor: "pointer", padding: "0 2px" };
          return (
            <div key={i} onClick={() => set({ layerIdx: i })} style={{
              display: "flex", alignItems: "center", gap: "3px",
              padding: "3px 8px", cursor: "pointer",
              background: isActive ? "rgba(220,160,20,0.12)" : "transparent",
              borderLeft: isActive ? "2px solid var(--amber)" : "2px solid transparent",
            }}>
              <button onClick={e => { e.stopPropagation(); toggleLayerVisible(i); }} style={{ ...iconBtn, opacity: layer.visible ? 1 : 0.35 }}>
                {layer.visible ? "◉" : "○"}
              </button>
              {editingLayerName === i ? (
                <input autoFocus defaultValue={layer.name}
                  onBlur={e => renameLayer(i, e.currentTarget.value || layer.name)}
                  onKeyDown={e => { if (e.key === "Enter") renameLayer(i, e.currentTarget.value || layer.name); e.stopPropagation(); }}
                  onClick={e => e.stopPropagation()}
                  style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: "10px", background: "var(--paper-2)", border: "1px solid var(--amber)", color: "var(--ink)", padding: "1px 3px", outline: "none" }}
                />
              ) : (
                <span onDoubleClick={e => { e.stopPropagation(); setEditingLayerName(i); }} style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: "10px", color: isActive ? "var(--ink)" : "var(--ink-3)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {layer.name || `Layer ${i + 1}`}
                </span>
              )}
              <button onClick={e => { e.stopPropagation(); moveLayer(i, 1); }} style={iconBtn} title="Move up">↑</button>
              <button onClick={e => { e.stopPropagation(); moveLayer(i, -1); }} style={iconBtn} title="Move down">↓</button>
              <button onClick={e => { e.stopPropagation(); removeLayer(i); }} disabled={comp.layers.length <= 1} style={{ ...iconBtn, opacity: comp.layers.length <= 1 ? 0.2 : 0.6 }}>×</button>
            </div>
          );
        })}
      </div>

      {/* Palette config (collapsible) */}
      <div style={{ borderTop: "1px solid var(--rule)" }}>
        <button
          onClick={() => setPalettesOpen(o => !o)}
          style={{ display: "flex", alignItems: "center", justifyContent: "space-between", width: "100%", padding: "6px 12px", background: "none", border: "none", cursor: "pointer" }}
        >
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)" }}>PALETTES ({comp.palettes.length})</span>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>{palettesOpen ? "▲" : "▼"}</span>
        </button>
        {palettesOpen && (
          <div style={{ padding: "0 12px 8px" }}>
            <div style={{ display: "flex", gap: "6px", marginBottom: "6px" }}>
              <button onClick={addPalette} style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "2px 8px", cursor: "pointer" }}>+ Add palette</button>
              <button onClick={() => setPickingPaletteFile(true)} style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "2px 8px", cursor: "pointer" }}>◧ Load .tilepal</button>
            </div>
            {comp.palettes.map((p, i) => (
              <div key={i} style={{ marginBottom: "8px", padding: "6px 8px", border: "1px solid var(--rule)", background: "rgba(0,0,0,0.15)" }}>
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "4px" }}>
                  <input
                    defaultValue={p.name}
                    placeholder={`Palette ${i + 1}`}
                    onBlur={e => patchPalette(i, { name: e.currentTarget.value })}
                    style={{ fontFamily: "var(--font-mono)", fontSize: "10px", background: "none", border: "none", borderBottom: "1px solid var(--rule-2)", color: "var(--ink-2)", width: "100px", outline: "none", padding: "1px 2px" }}
                  />
                  <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
                    {p.texture_path && onOpenPalette && (() => {
                      const stem = p.texture_path.replace(/\.[^.]+$/, "");
                      const palPath = `${stem}.tilepal`;
                      return (
                        <button
                          onClick={() => onOpenPalette(palPath)}
                          title="Edit palette"
                          style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "9px", cursor: "pointer", padding: "1px 4px", borderRadius: "2px" }}
                        >Edit</button>
                      );
                    })()}
                    <button onClick={() => removePalette(i)} style={{ background: "none", border: "none", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "14px", cursor: "pointer", padding: "0 2px" }}>×</button>
                  </div>
                </div>
                <BrowseInputField label="tex" value={p.texture_path} placeholder="(none)" onCommit={v => patchPalette(i, { texture_path: v })} onBrowse={() => setPickingPaletteTex(i)} />
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "4px", marginTop: "6px" }}>
                  {([
                    ["cols", p.tileset_cols, 1, (v: number) => patchPalette(i, { tileset_cols: Math.round(v) })],
                    ["rows", p.tileset_rows, 1, (v: number) => patchPalette(i, { tileset_rows: Math.round(v) })],
                    ["margin", p.margin ?? 0, 0, (v: number) => patchPalette(i, { margin: Math.round(v) })],
                    ["spacing", p.spacing ?? 0, 0, (v: number) => patchPalette(i, { spacing: Math.round(v) })],
                  ] as [string, number, number, (v: number) => void][]).map(([lbl, val, minVal, commit]) => (
                    <label key={lbl} style={{ display: "flex", flexDirection: "column", gap: "2px" }}>
                      <span style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)", textTransform: "uppercase" }}>{lbl}</span>
                      <input
                        type="number"
                        defaultValue={val}
                        min={minVal}
                        step={1}
                        key={`${lbl}-${val}`}
                        onBlur={e => { const n = parseInt(e.currentTarget.value, 10); if (!isNaN(n)) commit(Math.max(minVal, n)); }}
                        onKeyDown={e => { if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur(); }}
                        style={{ fontFamily: "var(--font-mono)", fontSize: "11px", background: "var(--paper-2)", border: "1px solid var(--rule-2)", color: "var(--ink)", padding: "2px 4px", width: "100%", boxSizing: "border-box", outline: "none" }}
                      />
                    </label>
                  ))}
                </div>
                {p.solid_tiles.length > 0 && (
                  <div style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "rgba(220,100,100,0.8)", marginTop: "3px" }}>
                    {p.solid_tiles.length} solid tile{p.solid_tiles.length !== 1 ? "s" : ""}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </>
  );
}
