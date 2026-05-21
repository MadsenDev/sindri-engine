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
  solid_tiles?: number[];
}

interface Props {
  comp: TilemapComp;
  entityId: number;
  componentIdx: number;
  onClose: () => void;
}

type PaintMode = "draw" | "erase" | "collision";

function resolveTextureUrl(path: string): string {
  if (!path) return "";
  if (path.startsWith("http") || path.startsWith("data:")) return path;
  return `http://localhost:7878/assets/${path}`;
}

function cellUV(col: number, row: number, cols: number, rows: number, margin: number, spacing: number, imgW: number, imgH: number) {
  if (margin === 0 && spacing === 0) {
    return { x: (col / cols) * 100, y: (row / rows) * 100, w: (1 / cols) * 100, h: (1 / rows) * 100 };
  }
  const cellW = (imgW - 2 * margin - spacing * (cols - 1)) / cols;
  const cellH = (imgH - 2 * margin - spacing * (rows - 1)) / rows;
  const px = margin + col * (cellW + spacing);
  const py = margin + row * (cellH + spacing);
  return { x: (px / imgW) * 100, y: (py / imgH) * 100, w: (cellW / imgW) * 100, h: (cellH / imgH) * 100 };
}

export default function TilePainter({ comp, entityId, componentIdx, onClose }: Props) {
  const [tiles, setTiles] = useState<number[]>(() => {
    const expected = comp.map_cols * comp.map_rows;
    const t = [...comp.tiles];
    while (t.length < expected) t.push(0);
    return t.slice(0, expected);
  });
  const [solidTiles, setSolidTiles] = useState<Set<number>>(
    () => new Set(comp.solid_tiles ?? [])
  );
  const [selectedTile, setSelectedTile] = useState<number>(1);
  const [paintMode, setPaintMode] = useState<PaintMode>("draw");
  const [isPainting, setIsPainting] = useState(false);
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  const pendingTilesRef = useRef<number[] | null>(null);
  const pendingSolidRef = useRef<Set<number> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!comp.texture_path) return;
    const el = new Image();
    el.onload = () => setImg(el);
    el.onerror = () => setImg(null);
    el.src = resolveTextureUrl(comp.texture_path);
  }, [comp.texture_path]);

  const flush = useCallback(async (latestTiles: number[] | null, latestSolid: Set<number> | null) => {
    const data: Record<string, unknown> = {};
    if (latestTiles) data.tiles = latestTiles;
    if (latestSolid) data.solid_tiles = Array.from(latestSolid);
    if (Object.keys(data).length === 0) return;
    try {
      await invoke("patch_component", { entityId, componentIdx, data });
    } catch (e) {
      console.error("tile save failed:", e);
    }
  }, [entityId, componentIdx]);

  const scheduleSave = useCallback((tiles: number[] | null, solid: Set<number> | null) => {
    if (tiles) pendingTilesRef.current = tiles;
    if (solid) pendingSolidRef.current = solid;
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      flush(pendingTilesRef.current, pendingSolidRef.current);
      pendingTilesRef.current = null;
      pendingSolidRef.current = null;
    }, 300);
  }, [flush]);

  const paintTile = useCallback((idx: number) => {
    if (paintMode === "collision") return; // collision mode handled via tileset click
    setTiles(prev => {
      if (idx < 0 || idx >= prev.length) return prev;
      const val = paintMode === "erase" ? 0 : selectedTile;
      if (prev[idx] === val) return prev;
      const next = [...prev];
      next[idx] = val;
      scheduleSave(next, null);
      return next;
    });
  }, [paintMode, selectedTile, scheduleSave]);

  const toggleSolid = useCallback((tileId: number) => {
    setSolidTiles(prev => {
      const next = new Set(prev);
      if (next.has(tileId)) next.delete(tileId);
      else next.add(tileId);
      scheduleSave(null, next);
      return next;
    });
  }, [scheduleSave]);

  const handleTilesetClick = useCallback((tileId: number) => {
    if (paintMode === "collision") {
      toggleSolid(tileId);
    } else {
      setSelectedTile(tileId);
      if (paintMode === "erase") setPaintMode("draw");
    }
  }, [paintMode, toggleSolid]);

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

  const floodFill = useCallback((startIdx: number, fillId: number) => {
    setTiles(prev => {
      const targetId = prev[startIdx];
      if (targetId === fillId) return prev;
      const next = [...prev];
      const cols = comp.map_cols;
      const rows = comp.map_rows;
      const stack = [startIdx];
      const visited = new Set<number>();
      while (stack.length > 0) {
        const idx = stack.pop()!;
        if (visited.has(idx) || idx < 0 || idx >= next.length) continue;
        if (next[idx] !== targetId) continue;
        visited.add(idx);
        next[idx] = fillId;
        const c = idx % cols;
        const r = Math.floor(idx / cols);
        if (c > 0) stack.push(idx - 1);
        if (c < cols - 1) stack.push(idx + 1);
        if (r > 0) stack.push(idx - cols);
        if (r < rows - 1) stack.push(idx + cols);
      }
      scheduleSave(next, null);
      return next;
    });
  }, [comp.map_cols, comp.map_rows, scheduleSave]);

  const handleMapClick = (e: React.MouseEvent, idx: number) => {
    if (e.shiftKey && paintMode !== "collision") {
      floodFill(idx, paintMode === "erase" ? 0 : selectedTile);
    }
  };

  const cols = Math.max(1, comp.tileset_cols);
  const rows = Math.max(1, comp.tileset_rows);
  const margin = comp.margin ?? 0;
  const spacing = comp.spacing ?? 0;
  const TILESET_CELL = Math.min(48, Math.floor(320 / cols));
  const MAP_CELL = Math.max(12, Math.min(32, Math.floor(500 / comp.map_cols)));

  const modeButtonStyle = (mode: PaintMode): React.CSSProperties => ({
    background: paintMode === mode
      ? mode === "collision" ? "rgba(220,50,50,0.8)" : "var(--amber)"
      : "none",
    border: `1px solid ${paintMode === mode ? "transparent" : "var(--rule-2)"}`,
    color: paintMode === mode ? "var(--paper)" : "var(--ink-3)",
    fontFamily: "var(--font-mono)", fontSize: "10px",
    padding: "3px 10px", cursor: "pointer",
  });

  return (
    <div
      style={{
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
        width: "min(90vw, 960px)", maxHeight: "90vh", overflow: "hidden",
      }}>
        {/* Header */}
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "10px 16px", borderBottom: "1px solid var(--rule)", flexShrink: 0,
        }}>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-3)" }}>
            TILE PAINTER · {comp.map_cols}×{comp.map_rows}
            {solidTiles.size > 0 && (
              <span style={{ color: "rgba(220,80,80,0.9)", marginLeft: "10px" }}>
                {solidTiles.size} solid type{solidTiles.size !== 1 ? "s" : ""}
              </span>
            )}
          </span>
          <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
            <button onClick={() => setPaintMode("draw")} style={modeButtonStyle("draw")}>Draw</button>
            <button onClick={() => setPaintMode("erase")} style={modeButtonStyle("erase")}>Erase</button>
            <button onClick={() => setPaintMode("collision")} style={modeButtonStyle("collision")} title="Click tiles in the tileset to mark them solid">Collision</button>
            <div style={{ width: 1, height: 16, background: "var(--rule-2)", margin: "0 2px" }} />
            <button
              onClick={() => {
                const cleared = new Array(comp.map_cols * comp.map_rows).fill(0);
                setTiles(cleared);
                flush(cleared, null);
              }}
              style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "3px 10px", cursor: "pointer" }}
            >Clear</button>
            <button
              onClick={onClose}
              style={{ background: "none", border: "none", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "18px", cursor: "pointer", padding: "0 4px" }}
            >×</button>
          </div>
        </div>

        {paintMode === "collision" && (
          <div style={{
            padding: "6px 16px", background: "rgba(200,50,50,0.12)",
            borderBottom: "1px solid rgba(200,50,50,0.3)", flexShrink: 0,
          }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "rgba(220,100,100,0.9)" }}>
              Click tiles in the tileset to toggle solid collision. Shift+click a map tile to flood fill.
            </span>
          </div>
        )}
        {paintMode !== "collision" && (
          <div style={{
            padding: "5px 16px", background: "rgba(0,0,0,0.1)",
            borderBottom: "1px solid var(--rule)", flexShrink: 0,
          }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
              Shift+click to flood fill · Tile #{selectedTile}
            </span>
          </div>
        )}

        <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
          {/* Left: tileset picker */}
          <div style={{
            width: "280px", flexShrink: 0, borderRight: "1px solid var(--rule)",
            display: "flex", flexDirection: "column", overflow: "hidden",
          }}>
            <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
                TILESET · {cols}×{rows}
                {paintMode === "collision" ? " · click to toggle solid" : ` · tile #${selectedTile}`}
              </span>
            </div>
            <div style={{ overflow: "auto", flex: 1, padding: "8px" }}>
              {comp.texture_path && img ? (
                <div style={{
                  display: "grid",
                  gridTemplateColumns: `repeat(${cols}, ${TILESET_CELL}px)`,
                  gap: 0,
                  width: `${cols * TILESET_CELL}px`,
                  position: "relative",
                }}>
                  {Array.from({ length: cols * rows }, (_, i) => {
                    const tc = i % cols;
                    const tr = Math.floor(i / cols);
                    const uv = cellUV(tc, tr, cols, rows, margin, spacing, img.naturalWidth, img.naturalHeight);
                    const tileId = i + 1;
                    const isSelected = selectedTile === tileId && paintMode !== "collision";
                    const isSolid = solidTiles.has(tileId);
                    return (
                      <div
                        key={i}
                        onClick={() => handleTilesetClick(tileId)}
                        title={paintMode === "collision" ? `Tile #${tileId} — ${isSolid ? "solid" : "passable"} (click to toggle)` : `Tile #${tileId}`}
                        style={{
                          width: TILESET_CELL, height: TILESET_CELL, position: "relative",
                          backgroundImage: `url(${resolveTextureUrl(comp.texture_path)})`,
                          backgroundPosition: `-${uv.x * TILESET_CELL / 100 * cols}px -${uv.y * TILESET_CELL / 100 * rows}px`,
                          backgroundSize: `${cols * TILESET_CELL}px ${rows * TILESET_CELL}px`,
                          cursor: "pointer",
                          outline: isSelected ? "2px solid var(--amber)" : isSolid ? "2px solid rgba(220,80,80,0.8)" : "1px solid rgba(255,255,255,0.05)",
                          outlineOffset: "-2px",
                          boxSizing: "border-box",
                          imageRendering: "pixelated",
                        }}
                      >
                        {isSolid && (
                          <div style={{
                            position: "absolute", inset: 0,
                            background: "rgba(220,60,60,0.25)",
                            pointerEvents: "none",
                          }} />
                        )}
                      </div>
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
                const isEmpty = tileId === 0;
                const isSolid = !isEmpty && solidTiles.has(tileId);
                const tc = isEmpty ? 0 : (tileId - 1) % cols;
                const tr = isEmpty ? 0 : Math.floor((tileId - 1) / cols);
                const uv = (!isEmpty && img) ? cellUV(tc, tr, cols, rows, margin, spacing, img.naturalWidth, img.naturalHeight) : null;
                return (
                  <div
                    key={idx}
                    onPointerDown={e => handleMapPointer(e, idx)}
                    onPointerEnter={() => handleMapEnter(idx)}
                    onClick={e => handleMapClick(e, idx)}
                    style={{
                      width: MAP_CELL, height: MAP_CELL, boxSizing: "border-box",
                      border: "1px solid rgba(255,255,255,0.06)",
                      cursor: paintMode === "collision" ? "default" : "crosshair",
                      backgroundImage: (!isEmpty && uv && comp.texture_path) ? `url(${resolveTextureUrl(comp.texture_path)})` : undefined,
                      backgroundPosition: uv ? `-${uv.x * MAP_CELL / 100 * cols}px -${uv.y * MAP_CELL / 100 * rows}px` : undefined,
                      backgroundSize: uv ? `${cols * MAP_CELL}px ${rows * MAP_CELL}px` : undefined,
                      backgroundColor: isEmpty ? "rgba(0,0,0,0.3)" : undefined,
                      imageRendering: "pixelated",
                      position: "relative",
                    }}
                  >
                    {isSolid && (
                      <div style={{
                        position: "absolute", inset: 0,
                        background: "rgba(220,60,60,0.35)",
                        pointerEvents: "none",
                      }} />
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
