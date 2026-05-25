import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TilePalette, TileColliderShape } from "../App";

interface Props {
  palettePath?: string;         // edit mode: load this .tilepal file
  initialTexturePath?: string;  // create mode: start from this image
  projectPath: string;
  onClose: () => void;
  onSaved?: (path: string, updated: TilePalette) => void;
}

const SHAPES: { key: TileColliderShape["type"]; label: string; unicode: string }[] = [
  { key: "none",         label: "None",     unicode: "○" },
  { key: "full",         label: "Full",     unicode: "■" },
  { key: "rect",         label: "Rect",     unicode: "▪" },
  { key: "slope_cut_tl", label: "╱ Floor",  unicode: "╱" },
  { key: "slope_cut_tr", label: "╲ Floor",  unicode: "╲" },
  { key: "slope_cut_bl", label: "╲ Ceil",   unicode: "⌐" },
  { key: "slope_cut_br", label: "╱ Ceil",   unicode: "¬" },
];

const OVERLAY_FILL   = "rgba(80,180,220,0.3)";
const OVERLAY_STROKE = "rgba(80,180,220,0.8)";
const DISABLED_FILL  = "rgba(0,0,0,0.55)";
const SELECTED_STROKE = "rgba(240,192,80,0.95)";
const SOLID_FILL     = "rgba(220,80,60,0.30)";
const SOLID_STROKE   = "rgba(220,80,60,0.65)";
const GRID_COLOR     = "rgba(255,255,255,0.07)";

function resolveUrl(path: string): string {
  if (!path || path.startsWith("http") || path.startsWith("data:")) return path;
  return `http://localhost:7878/assets/${path}`;
}

function defaultCollider(type: TileColliderShape["type"]): TileColliderShape {
  if (type === "rect") return { type: "rect", x: 0, y: 0, w: 1, h: 1 };
  return { type } as TileColliderShape;
}

function effectiveCollider(pal: TilePalette, idx: number): TileColliderShape {
  const custom = pal.tile_colliders?.[idx];
  if (custom) return custom;
  if (pal.solid_tiles.includes(idx)) return { type: "full" };
  return { type: "none" };
}

function drawShapeOverlay(
  ctx: CanvasRenderingContext2D,
  shape: TileColliderShape,
  x: number, y: number, w: number, h: number,
  fill: string, stroke: string,
) {
  if (shape.type === "none") return;
  ctx.fillStyle = fill;
  ctx.strokeStyle = stroke;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  if (shape.type === "full") {
    ctx.rect(x + 1, y + 1, w - 2, h - 2);
  } else if (shape.type === "rect") {
    ctx.rect(x + shape.x * w + 0.5, y + shape.y * h + 0.5, shape.w * w - 1, shape.h * h - 1);
  } else {
    const tl: [number, number] = [x, y];
    const tr: [number, number] = [x + w, y];
    const bl: [number, number] = [x, y + h];
    const br: [number, number] = [x + w, y + h];
    const verts: Record<string, [[number,number],[number,number],[number,number]]> = {
      slope_cut_tl: [tr, bl, br],
      slope_cut_tr: [tl, bl, br],
      slope_cut_bl: [tl, tr, br],
      slope_cut_br: [tl, tr, bl],
    };
    const pts = verts[shape.type];
    if (pts) {
      ctx.moveTo(pts[0][0], pts[0][1]);
      ctx.lineTo(pts[1][0], pts[1][1]);
      ctx.lineTo(pts[2][0], pts[2][1]);
      ctx.closePath();
    }
  }
  ctx.fill();
  ctx.stroke();
}

function makePaletteFromTexture(texturePath: string): TilePalette {
  const stem = texturePath.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "palette";
  return {
    name: stem,
    texture_path: texturePath,
    tileset_cols: 4,
    tileset_rows: 4,
    margin: 0,
    spacing: 0,
    solid_tiles: [],
    tile_colliders: {},
    disabled_tiles: [],
  };
}

// ─── Number spinner ──────────────────────────────────────────────────────────

function NumField({ label, value, min = 0, max = 512, onChange }: {
  label: string; value: number; min?: number; max?: number;
  onChange: (v: number) => void;
}) {
  return (
    <label style={{ display: "flex", alignItems: "center", gap: "6px", marginBottom: "5px" }}>
      <span style={{ width: "62px", fontSize: "11px", color: "var(--ink-3)", fontFamily: "var(--font-mono)", flexShrink: 0 }}>{label}</span>
      <div style={{ display: "flex", alignItems: "center", background: "var(--paper)", border: "1px solid var(--rule-2)", borderRadius: "3px", overflow: "hidden" }}>
        <button
          onMouseDown={e => { e.preventDefault(); onChange(Math.max(min, value - 1)); }}
          style={{ width: "20px", height: "22px", background: "none", border: "none", color: "var(--ink-3)", cursor: "pointer", fontSize: "12px", lineHeight: 1, flexShrink: 0 }}
        >−</button>
        <input
          type="number" min={min} max={max} value={value}
          onChange={e => { const v = parseInt(e.target.value); if (!isNaN(v)) onChange(Math.max(min, Math.min(max, v))); }}
          style={{ width: "38px", textAlign: "center", background: "none", border: "none", color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", outline: "none", padding: "0 2px" }}
        />
        <button
          onMouseDown={e => { e.preventDefault(); onChange(Math.min(max, value + 1)); }}
          style={{ width: "20px", height: "22px", background: "none", border: "none", color: "var(--ink-3)", cursor: "pointer", fontSize: "12px", lineHeight: 1, flexShrink: 0 }}
        >+</button>
      </div>
    </label>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

export default function TilePaletteEditor({
  palettePath,
  initialTexturePath,
  projectPath,
  onClose,
  onSaved,
}: Props) {
  const isCreate = !palettePath;
  const [palette, setPalette] = useState<TilePalette | null>(null);
  const [savePath, setSavePath] = useState<string | null>(null);
  const [selectedTiles, setSelectedTiles] = useState<Set<number>>(new Set());
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Initialize palette
  useEffect(() => {
    if (palettePath) {
      invoke<TilePalette>("read_tile_palette", { projectPath, relativePath: palettePath })
        .then(data => {
          setPalette({
            ...data,
            tile_colliders: data.tile_colliders ?? {},
            disabled_tiles: data.disabled_tiles ?? [],
          });
          setSavePath(palettePath);
        })
        .catch(console.error);
    } else if (initialTexturePath) {
      const p = makePaletteFromTexture(initialTexturePath);
      setPalette(p);
      setSavePath(initialTexturePath.replace(/\.[^.]+$/, "") + ".tilepal");
      setDirty(true);
    }
  }, [palettePath, initialTexturePath, projectPath]);

  // Load image when texture_path changes
  useEffect(() => {
    if (!palette?.texture_path) { setImg(null); return; }
    const el = new Image();
    el.onload = () => setImg(el);
    el.onerror = () => setImg(null);
    el.src = resolveUrl(palette.texture_path);
  }, [palette?.texture_path]);

  // Compute display cell size
  const cols = palette?.tileset_cols ?? 1;
  const rows = palette?.tileset_rows ?? 1;
  const margin = palette?.margin ?? 0;
  const spacing = palette?.spacing ?? 0;

  const tilePixW = img ? (img.naturalWidth  - 2 * margin - spacing * (cols - 1)) / cols : 16;
  const tilePixH = img ? (img.naturalHeight - 2 * margin - spacing * (rows - 1)) / rows : 16;
  const cellW = Math.min(80, Math.max(24, Math.floor(520 / Math.max(cols, 1))));
  const cellH = Math.round(cellW * tilePixH / Math.max(tilePixW, 1));

  // Render canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !palette || !img) return;

    canvas.width  = cols * cellW;
    canvas.height = rows * cellH;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.imageSmoothingEnabled = false;

    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        const idx = r * cols + c;
        const dx = c * cellW;
        const dy = r * cellH;
        const sx = margin + c * (tilePixW + spacing);
        const sy = margin + r * (tilePixH + spacing);

        // Draw tile image
        ctx.drawImage(img, sx, sy, tilePixW, tilePixH, dx, dy, cellW, cellH);

        const shape = effectiveCollider(palette, idx);
        const disabled = palette.disabled_tiles?.includes(idx);
        const selected = selectedTiles.has(idx);

        // Collision overlay
        if (shape.type !== "none") {
          drawShapeOverlay(ctx, shape, dx, dy, cellW, cellH, SOLID_FILL, SOLID_STROKE);
        }

        // Disabled overlay
        if (disabled) {
          ctx.fillStyle = DISABLED_FILL;
          ctx.fillRect(dx, dy, cellW, cellH);
          ctx.strokeStyle = "rgba(255,80,80,0.65)";
          ctx.lineWidth = 1.5;
          ctx.beginPath();
          ctx.moveTo(dx + 5, dy + 5); ctx.lineTo(dx + cellW - 5, dy + cellH - 5);
          ctx.moveTo(dx + cellW - 5, dy + 5); ctx.lineTo(dx + 5, dy + cellH - 5);
          ctx.stroke();
        }

        // Selection
        if (selected) {
          ctx.fillStyle = OVERLAY_FILL;
          ctx.fillRect(dx, dy, cellW, cellH);
          ctx.strokeStyle = SELECTED_STROKE;
          ctx.lineWidth = 2;
          ctx.strokeRect(dx + 1, dy + 1, cellW - 2, cellH - 2);
        }

        // Grid line
        ctx.strokeStyle = GRID_COLOR;
        ctx.lineWidth = 0.5;
        ctx.strokeRect(dx + 0.5, dy + 0.5, cellW - 1, cellH - 1);
      }
    }
  }, [palette, img, selectedTiles, cols, rows, margin, spacing, cellW, cellH, tilePixW, tilePixH]);

  const handleCanvasClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!palette) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const col = Math.floor((e.clientX - rect.left) / cellW);
    const row = Math.floor((e.clientY - rect.top)  / cellH);
    if (col < 0 || col >= cols || row < 0 || row >= rows) return;
    const idx = row * cols + col;

    if (e.shiftKey) {
      setSelectedTiles(prev => {
        const next = new Set(prev);
        if (next.has(idx)) next.delete(idx); else next.add(idx);
        return next;
      });
    } else {
      setSelectedTiles(new Set([idx]));
    }
  }, [palette, cols, rows, cellW, cellH]);

  const update = useCallback((fn: (p: TilePalette) => TilePalette) => {
    setPalette(prev => prev ? fn(prev) : prev);
    setDirty(true);
  }, []);

  const setShapeForSelected = (shapeType: TileColliderShape["type"]) => {
    update(p => {
      const colliders = { ...p.tile_colliders };
      for (const idx of selectedTiles) {
        if (shapeType === "none") delete colliders[idx];
        else colliders[idx] = defaultCollider(shapeType);
      }
      const solidSet = new Set(p.solid_tiles);
      for (const idx of selectedTiles) {
        if (shapeType === "full") solidSet.add(idx); else solidSet.delete(idx);
      }
      return { ...p, tile_colliders: colliders, solid_tiles: Array.from(solidSet).sort((a, b) => a - b) };
    });
  };

  const patchRectForSelected = (fields: Partial<{ x: number; y: number; w: number; h: number }>) => {
    update(p => {
      const colliders = { ...p.tile_colliders };
      for (const idx of selectedTiles) {
        const cur = colliders[idx];
        if (cur?.type === "rect") colliders[idx] = { ...cur, ...fields };
      }
      return { ...p, tile_colliders: colliders };
    });
  };

  const toggleDisabledForSelected = () => {
    update(p => {
      const disabled = new Set(p.disabled_tiles ?? []);
      const anyEnabled = [...selectedTiles].some(i => !disabled.has(i));
      for (const idx of selectedTiles) {
        if (anyEnabled) disabled.add(idx); else disabled.delete(idx);
      }
      return { ...p, disabled_tiles: Array.from(disabled).sort((a, b) => a - b) };
    });
  };

  const handleSave = async () => {
    if (!palette || !savePath) return;
    setSaving(true);
    try {
      await invoke("write_tile_palette", { projectPath, relativePath: savePath, data: palette });
      setDirty(false);
      onSaved?.(savePath, palette);
      if (isCreate) onClose();
    } catch (e) {
      console.error("write_tile_palette failed:", e);
    } finally {
      setSaving(false);
    }
  };

  // Derived selection info
  const selectedArr = Array.from(selectedTiles);
  const firstSelected = selectedArr.length > 0 && palette ? effectiveCollider(palette, selectedArr[0]) : null;
  const allSameShape = selectedArr.length > 0 && palette &&
    selectedArr.every(i => effectiveCollider(palette, i).type === firstSelected?.type);
  const activeShapeType = allSameShape ? firstSelected?.type : null;
  const rectShape = activeShapeType === "rect" && selectedArr.length === 1 && palette
    ? (effectiveCollider(palette, selectedArr[0]) as Extract<TileColliderShape, { type: "rect" }>)
    : null;

  const title = isCreate
    ? "New Tile Palette"
    : (savePath?.split("/").pop() ?? "Tile Palette");

  if (!palette) {
    return (
      <div style={overlayStyle}>
        <div style={modalStyle}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", flex: 1, color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "12px" }}>
            Loading…
          </div>
        </div>
      </div>
    );
  }

  const canSave = dirty && !saving && !!palette.texture_path && cols > 0 && rows > 0;

  return (
    <div style={overlayStyle} onMouseDown={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={modalStyle}>

        {/* ── Header ──────────────────────────────────────────────── */}
        <div style={{
          height: "46px", display: "flex", alignItems: "center", gap: "10px",
          padding: "0 18px", borderBottom: "1px solid var(--rule)", flexShrink: 0,
        }}>
          <span style={{ fontSize: "13px", color: "var(--ink-3)" }}>◧</span>
          <span style={{ fontFamily: "var(--font-ui)", fontSize: "14px", color: "var(--ink)", fontWeight: 500 }}>
            {title}
          </span>
          {savePath && (
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
              → {savePath}
            </span>
          )}
          <div style={{ flex: 1 }} />
          {dirty && (
            <button
              onClick={handleSave}
              disabled={!canSave}
              style={{
                padding: "4px 14px", background: canSave ? "var(--moss)" : "var(--paper-3)",
                border: "none", borderRadius: "3px", color: canSave ? "#fff" : "var(--ink-4)",
                fontFamily: "var(--font-mono)", fontSize: "11px", cursor: canSave ? "pointer" : "default",
              }}
            >
              {saving ? "Saving…" : isCreate ? "Create" : "Save"}
            </button>
          )}
          <button onClick={onClose} style={closeBtnStyle}>×</button>
        </div>

        {/* ── Body ────────────────────────────────────────────────── */}
        <div style={{ flex: 1, display: "flex", minHeight: 0, overflow: "hidden" }}>

          {/* Left: tileset canvas */}
          <div
            ref={containerRef}
            style={{ flex: 1, overflow: "auto", padding: "16px", background: "var(--paper)" }}
          >
            {!palette.texture_path ? (
              <div style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px", marginTop: "8px" }}>
                Set a texture path in the settings panel →
              </div>
            ) : !img ? (
              <div style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px", marginTop: "8px" }}>
                Loading texture… (engine must be running)
              </div>
            ) : (
              <>
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", marginBottom: "10px" }}>
                  click to select tile · shift+click for multi-select
                </div>
                <canvas
                  ref={canvasRef}
                  onClick={handleCanvasClick}
                  style={{ cursor: "crosshair", imageRendering: "pixelated", display: "block" }}
                />
              </>
            )}
          </div>

          {/* Right: settings + per-tile inspector */}
          <div style={{
            width: "230px", flexShrink: 0, borderLeft: "1px solid var(--rule)",
            display: "flex", flexDirection: "column", overflow: "hidden",
          }}>
            {/* Palette settings */}
            <div style={{ padding: "12px 14px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)", textTransform: "uppercase", marginBottom: "8px", letterSpacing: "0.06em" }}>
                Palette Settings
              </div>

              <label style={{ display: "flex", alignItems: "center", gap: "6px", marginBottom: "5px" }}>
                <span style={{ width: "62px", fontSize: "11px", color: "var(--ink-3)", fontFamily: "var(--font-mono)", flexShrink: 0 }}>name</span>
                <input
                  value={palette.name}
                  onChange={e => update(p => ({ ...p, name: e.target.value }))}
                  style={textInputStyle}
                />
              </label>

              <label style={{ display: "flex", alignItems: "flex-start", gap: "6px", marginBottom: "5px" }}>
                <span style={{ width: "62px", fontSize: "11px", color: "var(--ink-3)", fontFamily: "var(--font-mono)", flexShrink: 0, paddingTop: "3px" }}>texture</span>
                <input
                  value={palette.texture_path}
                  onChange={e => update(p => ({ ...p, texture_path: e.target.value }))}
                  placeholder="assets/tileset.png"
                  style={{ ...textInputStyle, fontSize: "9px" }}
                />
              </label>

              <div style={{ height: "1px", background: "var(--rule)", margin: "8px 0" }} />

              <NumField label="columns" value={palette.tileset_cols} min={1} max={128}
                onChange={v => update(p => ({ ...p, tileset_cols: v }))} />
              <NumField label="rows" value={palette.tileset_rows} min={1} max={128}
                onChange={v => update(p => ({ ...p, tileset_rows: v }))} />
              <NumField label="margin" value={palette.margin ?? 0} min={0} max={64}
                onChange={v => update(p => ({ ...p, margin: v }))} />
              <NumField label="spacing" value={palette.spacing ?? 0} min={0} max={64}
                onChange={v => update(p => ({ ...p, spacing: v }))} />

              {img && (
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)", marginTop: "6px" }}>
                  {Math.round(tilePixW)} × {Math.round(tilePixH)} px/tile · {cols * rows} tiles
                </div>
              )}
            </div>

            {/* Per-tile inspector */}
            <div style={{ flex: 1, overflow: "auto", padding: "10px 14px" }}>
              {selectedArr.length === 0 ? (
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", lineHeight: 1.7 }}>
                  Click a tile to edit<br />
                  its collision or hide<br />
                  it from the painter.<br /><br />
                  Shift+click to select<br />
                  multiple tiles.
                </div>
              ) : (
                <>
                  <div style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)", marginBottom: "8px", textTransform: "uppercase", letterSpacing: "0.06em" }}>
                    {selectedArr.length === 1 ? `Tile #${selectedArr[0]}` : `${selectedArr.length} tiles`}
                  </div>

                  {/* Collision shape picker */}
                  <div style={{ marginBottom: "10px" }}>
                    <div style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)", marginBottom: "5px", textTransform: "uppercase" }}>Collision</div>
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "3px" }}>
                      {SHAPES.map(({ key, label, unicode }) => (
                        <button
                          key={key}
                          onClick={() => setShapeForSelected(key)}
                          style={{
                            display: "flex", alignItems: "center", gap: "5px",
                            padding: "4px 7px",
                            background: activeShapeType === key ? "var(--paper-3)" : "var(--paper-2)",
                            border: `1px solid ${activeShapeType === key ? "var(--cyan)" : "var(--rule)"}`,
                            color: activeShapeType === key ? "var(--cyan)" : "var(--ink-3)",
                            fontFamily: "var(--font-mono)", fontSize: "10px",
                            cursor: "pointer", borderRadius: "3px", textAlign: "left",
                          }}
                        >
                          <span style={{ width: "12px", textAlign: "center" }}>{unicode}</span>
                          {label}
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* Rect sub-fields */}
                  {rectShape && (
                    <div style={{ marginBottom: "10px" }}>
                      <div style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)", marginBottom: "5px", textTransform: "uppercase" }}>Rect (0–1)</div>
                      {(["x", "y", "w", "h"] as const).map(field => (
                        <label key={field} style={{ display: "flex", alignItems: "center", gap: "6px", marginBottom: "4px" }}>
                          <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-3)", width: "12px" }}>{field}</span>
                          <input
                            type="number" min={0} max={1} step={0.05}
                            value={rectShape[field]}
                            onChange={e => patchRectForSelected({ [field]: parseFloat(e.target.value) || 0 })}
                            style={numInputStyle}
                          />
                        </label>
                      ))}
                    </div>
                  )}

                  {/* Disabled toggle */}
                  <div>
                    <div style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)", marginBottom: "5px", textTransform: "uppercase" }}>Visibility</div>
                    <button
                      onClick={toggleDisabledForSelected}
                      style={{
                        width: "100%", padding: "4px 8px",
                        background: selectedArr.some(i => palette.disabled_tiles?.includes(i)) ? "var(--paper-3)" : "var(--paper-2)",
                        border: `1px solid ${selectedArr.some(i => palette.disabled_tiles?.includes(i)) ? "rgba(220,80,60,0.6)" : "var(--rule)"}`,
                        color: selectedArr.some(i => palette.disabled_tiles?.includes(i)) ? "rgba(220,80,60,0.9)" : "var(--ink-3)",
                        fontFamily: "var(--font-mono)", fontSize: "10px",
                        cursor: "pointer", borderRadius: "3px", textAlign: "left",
                      }}
                    >
                      {selectedArr.some(i => palette.disabled_tiles?.includes(i)) ? "⊘ Hidden in painter" : "○ Visible in painter"}
                    </button>
                  </div>
                </>
              )}
            </div>

            {/* Legend */}
            <div style={{ padding: "8px 14px", borderTop: "1px solid var(--rule)", flexShrink: 0 }}>
              {[
                { color: SOLID_FILL, border: SOLID_STROKE, label: "Has collision" },
                { color: DISABLED_FILL, border: "rgba(255,80,80,0.4)", label: "Hidden in painter" },
                { color: OVERLAY_FILL, border: OVERLAY_STROKE, label: "Selected" },
              ].map(({ color, border, label }) => (
                <div key={label} style={{ display: "flex", alignItems: "center", gap: "6px", marginBottom: "3px" }}>
                  <div style={{ width: 12, height: 12, background: color, border: `1px solid ${border}`, flexShrink: 0 }} />
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)" }}>{label}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── Styles ───────────────────────────────────────────────────────────────────

const overlayStyle: React.CSSProperties = {
  position: "fixed", inset: 0,
  background: "rgba(0,0,0,0.72)",
  display: "flex", alignItems: "center", justifyContent: "center",
  zIndex: 300,
};

const modalStyle: React.CSSProperties = {
  width: "min(980px, 95vw)",
  height: "min(680px, 90vh)",
  background: "var(--paper-2)",
  border: "1px solid var(--rule-2)",
  boxShadow: "0 32px 80px rgba(0,0,0,0.55)",
  display: "flex", flexDirection: "column",
  overflow: "hidden",
};

const closeBtnStyle: React.CSSProperties = {
  background: "none", border: "none",
  color: "var(--ink-3)", fontSize: "18px",
  cursor: "pointer", lineHeight: 1, padding: "0 2px",
};

const textInputStyle: React.CSSProperties = {
  flex: 1, fontFamily: "var(--font-mono)", fontSize: "11px",
  background: "var(--paper)", border: "1px solid var(--rule-2)",
  color: "var(--ink)", padding: "2px 5px", outline: "none", borderRadius: "2px",
  minWidth: 0,
};

const numInputStyle: React.CSSProperties = {
  flex: 1, fontFamily: "var(--font-mono)", fontSize: "11px",
  background: "var(--paper)", border: "1px solid var(--rule-2)",
  color: "var(--ink)", padding: "2px 4px", outline: "none",
};
