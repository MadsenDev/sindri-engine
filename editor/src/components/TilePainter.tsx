import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TilemapComp {
  texture_path: string;
  tileset_cols: number;
  tileset_rows: number;
  tile_width: number;
  tile_height: number;
  map_cols: number;
  map_rows: number;
  tiles: number[];
  tint: [number, number, number, number];
  margin?: number;
  spacing?: number;
}

interface Props {
  comp: TilemapComp;
  entityId: number;
  componentIdx: number;
  onClose: () => void;
}

function resolveTextureUrl(path: string): string {
  if (!path) return "";
  if (path.startsWith("http") || path.startsWith("data:")) return path;
  return `http://localhost:7878/assets/${path}`;
}

// Compute UV for a cell accounting for margin and spacing
function cellUV(col: number, row: number, cols: number, rows: number, margin: number, spacing: number, imgW: number, imgH: number) {
  if (margin === 0 && spacing === 0) {
    return { x: (col / cols) * 100, y: (row / rows) * 100, w: (1 / cols) * 100, h: (1 / rows) * 100 };
  }
  const m = margin;
  const s = spacing;
  const cellW = (imgW - 2 * m - s * (cols - 1)) / cols;
  const cellH = (imgH - 2 * m - s * (rows - 1)) / rows;
  const px = m + col * (cellW + s);
  const py = m + row * (cellH + s);
  return { x: (px / imgW) * 100, y: (py / imgH) * 100, w: (cellW / imgW) * 100, h: (cellH / imgH) * 100 };
}

export default function TilePainter({ comp, entityId, componentIdx, onClose }: Props) {
  const [tiles, setTiles] = useState<number[]>(() => {
    const expected = comp.map_cols * comp.map_rows;
    const t = [...comp.tiles];
    while (t.length < expected) t.push(0);
    return t.slice(0, expected);
  });
  const [selectedTile, setSelectedTile] = useState<number>(1);
  const [isPainting, setIsPainting] = useState(false);
  const [paintMode, setPaintMode] = useState<"draw" | "erase">("draw");
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  const pendingRef = useRef<number[] | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!comp.texture_path) return;
    const el = new Image();
    el.onload = () => setImg(el);
    el.onerror = () => setImg(null);
    el.src = resolveTextureUrl(comp.texture_path);
  }, [comp.texture_path]);

  const flushSave = useCallback(async (latest: number[]) => {
    try {
      await invoke("patch_component", { entityId, componentIdx, data: { tiles: latest } });
    } catch (e) {
      console.error("tile save failed:", e);
    }
  }, [entityId, componentIdx]);

  const scheduleSave = useCallback((latest: number[]) => {
    pendingRef.current = latest;
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      if (pendingRef.current) flushSave(pendingRef.current);
    }, 300);
  }, [flushSave]);

  const paintTile = useCallback((idx: number) => {
    setTiles(prev => {
      if (idx < 0 || idx >= prev.length) return prev;
      const val = paintMode === "erase" ? 0 : selectedTile;
      if (prev[idx] === val) return prev;
      const next = [...prev];
      next[idx] = val;
      scheduleSave(next);
      return next;
    });
  }, [paintMode, selectedTile, scheduleSave]);

  const handleMapPointer = (e: React.PointerEvent<HTMLDivElement>, idx: number) => {
    if (e.buttons === 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    setIsPainting(true);
    paintTile(idx);
  };

  const handleMapEnter = (idx: number) => {
    if (!isPainting) return;
    paintTile(idx);
  };

  const stopPainting = () => setIsPainting(false);

  const cols = Math.max(1, comp.tileset_cols);
  const rows = Math.max(1, comp.tileset_rows);
  const margin = comp.margin ?? 0;
  const spacing = comp.spacing ?? 0;

  const TILESET_CELL = Math.min(48, Math.floor(320 / cols));
  const MAP_CELL = Math.max(12, Math.min(32, Math.floor(500 / comp.map_cols)));

  return (
    <div style={{
      position: "fixed", inset: 0, zIndex: 9000,
      background: "rgba(0,0,0,0.7)",
      display: "flex", alignItems: "center", justifyContent: "center",
    }}
      onMouseUp={stopPainting}
      onPointerUp={stopPainting}
    >
      <div style={{
        background: "var(--paper)", border: "1px solid var(--rule-2)",
        display: "flex", flexDirection: "column",
        width: "min(90vw, 960px)", maxHeight: "90vh",
        overflow: "hidden",
      }}>
        {/* Header */}
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "10px 16px", borderBottom: "1px solid var(--rule)",
          flexShrink: 0,
        }}>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-3)" }}>
            TILE PAINTER · {comp.map_cols}×{comp.map_rows}
          </span>
          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <button
              onClick={() => setPaintMode(m => m === "draw" ? "erase" : "draw")}
              style={{
                background: paintMode === "erase" ? "var(--red, #e05)" : "none",
                border: "1px solid var(--rule-2)", color: "var(--ink-3)",
                fontFamily: "var(--font-mono)", fontSize: "10px", padding: "3px 10px", cursor: "pointer",
              }}
            >
              {paintMode === "erase" ? "Erase" : "Draw"}
            </button>
            <button
              onClick={() => {
                const cleared = new Array(comp.map_cols * comp.map_rows).fill(0);
                setTiles(cleared);
                flushSave(cleared);
              }}
              style={{
                background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-4)",
                fontFamily: "var(--font-mono)", fontSize: "10px", padding: "3px 10px", cursor: "pointer",
              }}
            >
              Clear
            </button>
            <button
              onClick={onClose}
              style={{
                background: "none", border: "none", color: "var(--ink-3)",
                fontFamily: "var(--font-mono)", fontSize: "18px", cursor: "pointer", padding: "0 4px",
              }}
            >×</button>
          </div>
        </div>

        <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
          {/* Left: tileset picker */}
          <div style={{
            width: "280px", flexShrink: 0,
            borderRight: "1px solid var(--rule)",
            display: "flex", flexDirection: "column",
            overflow: "hidden",
          }}>
            <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
                TILESET · {cols}×{rows} · tile #{selectedTile}
              </span>
            </div>
            <div style={{ overflow: "auto", flex: 1, padding: "8px" }}>
              {comp.texture_path && img ? (
                <div style={{
                  display: "grid",
                  gridTemplateColumns: `repeat(${cols}, ${TILESET_CELL}px)`,
                  gap: 0,
                  position: "relative",
                  width: `${cols * TILESET_CELL}px`,
                }}>
                  {Array.from({ length: cols * rows }, (_, i) => {
                    const tc = i % cols;
                    const tr = Math.floor(i / cols);
                    const uv = cellUV(tc, tr, cols, rows, margin, spacing, img.naturalWidth, img.naturalHeight);
                    const tileId = i + 1;
                    const isSelected = selectedTile === tileId;
                    return (
                      <div
                        key={i}
                        onClick={() => { setSelectedTile(tileId); setPaintMode("draw"); }}
                        title={`Tile #${tileId}`}
                        style={{
                          width: TILESET_CELL, height: TILESET_CELL,
                          backgroundImage: `url(${resolveTextureUrl(comp.texture_path)})`,
                          backgroundPosition: `-${uv.x * TILESET_CELL / 100 * cols}px -${uv.y * TILESET_CELL / 100 * rows}px`,
                          backgroundSize: `${cols * TILESET_CELL}px ${rows * TILESET_CELL}px`,
                          cursor: "pointer",
                          outline: isSelected ? "2px solid var(--amber)" : "1px solid rgba(255,255,255,0.05)",
                          outlineOffset: isSelected ? "-2px" : "-1px",
                          boxSizing: "border-box",
                          imageRendering: "pixelated",
                        }}
                      />
                    );
                  })}
                </div>
              ) : (
                <div style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "12px 0" }}>
                  {comp.texture_path ? "loading tileset…" : "no texture set"}
                </div>
              )}
            </div>
          </div>

          {/* Right: map grid */}
          <div style={{ flex: 1, overflow: "auto", padding: "8px 12px" }}>
            <div style={{
              display: "grid",
              gridTemplateColumns: `repeat(${comp.map_cols}, ${MAP_CELL}px)`,
              gap: 0,
              width: `${comp.map_cols * MAP_CELL}px`,
              userSelect: "none",
            }}>
              {tiles.map((tileId, idx) => {
                const mc = idx % comp.map_cols;
                const mr = Math.floor(idx / comp.map_cols);
                const isEmpty = tileId === 0;
                const tc = isEmpty ? 0 : (tileId - 1) % cols;
                const tr = isEmpty ? 0 : Math.floor((tileId - 1) / cols);
                const uv = isEmpty ? null : (img ? cellUV(tc, tr, cols, rows, margin, spacing, img.naturalWidth, img.naturalHeight) : null);
                return (
                  <div
                    key={`${mc}-${mr}`}
                    onPointerDown={e => handleMapPointer(e, idx)}
                    onPointerEnter={() => handleMapEnter(idx)}
                    style={{
                      width: MAP_CELL, height: MAP_CELL,
                      boxSizing: "border-box",
                      border: "1px solid rgba(255,255,255,0.06)",
                      cursor: "crosshair",
                      backgroundImage: (!isEmpty && uv && comp.texture_path) ? `url(${resolveTextureUrl(comp.texture_path)})` : undefined,
                      backgroundPosition: uv ? `-${uv.x * MAP_CELL / 100 * cols}px -${uv.y * MAP_CELL / 100 * rows}px` : undefined,
                      backgroundSize: uv ? `${cols * MAP_CELL}px ${rows * MAP_CELL}px` : undefined,
                      backgroundColor: isEmpty ? "rgba(0,0,0,0.3)" : undefined,
                      imageRendering: "pixelated",
                    }}
                  />
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
