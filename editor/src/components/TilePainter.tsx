import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// u32 tile encoding matching Rust: upper 16 bits = palette_id (1-indexed), lower 16 bits = tile_idx (0-indexed)
function encodeTile(paletteId: number, tileIdx: number): number {
  return (((paletteId & 0xffff) << 16) | (tileIdx & 0xffff)) >>> 0;
}
function decodeTile(v: number): { paletteId: number; tileIdx: number } {
  return { paletteId: (v >>> 16) & 0xffff, tileIdx: v & 0xffff };
}

interface TilePalette {
  name: string;
  texture_path: string;
  tileset_cols: number;
  tileset_rows: number;
  margin: number;
  spacing: number;
  solid_tiles: number[];
}

interface TilemapComp {
  palettes: TilePalette[];
  tile_width: number;
  tile_height: number;
  map_cols: number;
  map_rows: number;
  tiles: number[];
  tint: [number, number, number, number];
}

interface Props {
  comp: TilemapComp;
  entityId: number;
  componentIdx: number;
  onClose: () => void;
}

type PaintMode = "draw" | "erase" | "collision";

function resolveTextureUrl(path: string): string {
  if (!path || path.startsWith("http") || path.startsWith("data:")) return path;
  return `http://localhost:7878/assets/${path}`;
}

function drawTileOnCanvas(
  ctx: CanvasRenderingContext2D,
  img: HTMLImageElement,
  tileIdx: number,
  palette: TilePalette,
  destX: number, destY: number, destW: number, destH: number,
) {
  const cols = Math.max(1, palette.tileset_cols);
  const rows = Math.max(1, palette.tileset_rows);
  const m = palette.margin ?? 0;
  const s = palette.spacing ?? 0;
  const iw = img.naturalWidth;
  const ih = img.naturalHeight;
  let srcX: number, srcY: number, srcW: number, srcH: number;
  if (m === 0 && s === 0) {
    srcW = iw / cols;
    srcH = ih / rows;
    srcX = (tileIdx % cols) * srcW;
    srcY = Math.floor(tileIdx / cols) * srcH;
  } else {
    srcW = (iw - 2 * m - s * (cols - 1)) / cols;
    srcH = (ih - 2 * m - s * (rows - 1)) / rows;
    srcX = m + (tileIdx % cols) * (srcW + s);
    srcY = m + Math.floor(tileIdx / cols) * (srcH + s);
  }
  ctx.drawImage(img, srcX, srcY, srcW, srcH, destX, destY, destW, destH);
}

export default function TilePainter({ comp, entityId, componentIdx, onClose }: Props) {
  const [tiles, setTiles] = useState<number[]>(() => {
    const expected = comp.map_cols * comp.map_rows;
    const t = [...comp.tiles];
    while (t.length < expected) t.push(0);
    return t.slice(0, expected);
  });
  const [palettes, setPalettes] = useState<TilePalette[]>(() =>
    comp.palettes.map(p => ({ ...p, solid_tiles: [...(p.solid_tiles ?? [])] }))
  );
  const [selectedPaletteIdx, setSelectedPaletteIdx] = useState(0);
  const [selectedTileIdx, setSelectedTileIdx] = useState(0);
  const [paintMode, setPaintMode] = useState<PaintMode>("draw");
  const [paletteImages, setPaletteImages] = useState<Map<string, HTMLImageElement>>(new Map());
  const [pan, setPan] = useState({ x: 16, y: 16 });
  const [zoom, setZoom] = useState(1.0);
  const [canvasSize, setCanvasSize] = useState({ w: 800, h: 600 });

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const canvasContainerRef = useRef<HTMLDivElement>(null);
  const isPainting = useRef(false);
  const panStartRef = useRef<{ mx: number; my: number; px: number; py: number } | null>(null);
  const spaceHeld = useRef(false);
  const loadedPathsRef = useRef<Set<string>>(new Set());

  const pendingTilesRef = useRef<number[] | null>(null);
  const pendingPalettesRef = useRef<TilePalette[] | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Resize observer
  useEffect(() => {
    const container = canvasContainerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;
    const ro = new ResizeObserver(entries => {
      for (const entry of entries) {
        const w = Math.floor(entry.contentRect.width);
        const h = Math.floor(entry.contentRect.height);
        canvas.width = w;
        canvas.height = h;
        setCanvasSize({ w, h });
      }
    });
    ro.observe(container);
    return () => ro.disconnect();
  }, []);

  // Load palette images
  useEffect(() => {
    palettes.forEach(p => {
      if (!p.texture_path || loadedPathsRef.current.has(p.texture_path)) return;
      loadedPathsRef.current.add(p.texture_path);
      const img = new Image();
      img.onload = () => setPaletteImages(prev => new Map(prev).set(p.texture_path, img));
      img.src = resolveTextureUrl(p.texture_path);
    });
  }, [palettes]);

  // Space key for pan
  useEffect(() => {
    const down = (e: KeyboardEvent) => { if (e.code === "Space" && e.target === document.body) { spaceHeld.current = true; e.preventDefault(); } };
    const up = (e: KeyboardEvent) => { if (e.code === "Space") spaceHeld.current = false; };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => { window.removeEventListener("keydown", down); window.removeEventListener("keyup", up); };
  }, []);

  const flush = useCallback(async (t: number[] | null, p: TilePalette[] | null) => {
    const data: Record<string, unknown> = {};
    if (t !== null) data.tiles = t;
    if (p !== null) data.palettes = p;
    if (Object.keys(data).length === 0) return;
    try { await invoke("patch_component", { entityId, componentIdx, data }); }
    catch (e) { console.error("tile save failed:", e); }
  }, [entityId, componentIdx]);

  const scheduleSave = useCallback((newTiles: number[] | null, newPalettes: TilePalette[] | null) => {
    if (newTiles !== null) pendingTilesRef.current = newTiles;
    if (newPalettes !== null) pendingPalettesRef.current = newPalettes;
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      flush(pendingTilesRef.current, pendingPalettesRef.current);
      pendingTilesRef.current = null;
      pendingPalettesRef.current = null;
    }, 300);
  }, [flush]);

  // Draw canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "rgba(28,28,33,1)";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    const tw = comp.tile_width * zoom;
    const th = comp.tile_height * zoom;

    // Grid lines
    ctx.strokeStyle = "rgba(255,255,255,0.07)";
    ctx.lineWidth = 0.5;
    for (let col = 0; col <= comp.map_cols; col++) {
      const x = pan.x + col * tw;
      ctx.beginPath(); ctx.moveTo(x, pan.y); ctx.lineTo(x, pan.y + comp.map_rows * th); ctx.stroke();
    }
    for (let row = 0; row <= comp.map_rows; row++) {
      const y = pan.y + row * th;
      ctx.beginPath(); ctx.moveTo(pan.x, y); ctx.lineTo(pan.x + comp.map_cols * tw, y); ctx.stroke();
    }

    // Tiles
    ctx.imageSmoothingEnabled = false;
    for (let row = 0; row < comp.map_rows; row++) {
      for (let col = 0; col < comp.map_cols; col++) {
        const cell = tiles[row * comp.map_cols + col] ?? 0;
        if (cell === 0) continue;
        const { paletteId, tileIdx } = decodeTile(cell);
        if (paletteId === 0) continue;
        const pal = palettes[paletteId - 1];
        if (!pal) continue;
        const img = paletteImages.get(pal.texture_path);
        if (!img) continue;
        drawTileOnCanvas(ctx, img, tileIdx, pal, pan.x + col * tw, pan.y + row * th, tw, th);
      }
    }

    // Solid overlays in collision mode
    if (paintMode === "collision") {
      ctx.fillStyle = "rgba(220,60,60,0.35)";
      for (let row = 0; row < comp.map_rows; row++) {
        for (let col = 0; col < comp.map_cols; col++) {
          const cell = tiles[row * comp.map_cols + col] ?? 0;
          if (cell === 0) continue;
          const { paletteId, tileIdx } = decodeTile(cell);
          if (paletteId === 0) continue;
          const pal = palettes[paletteId - 1];
          if (pal?.solid_tiles.includes(tileIdx)) {
            ctx.fillRect(pan.x + col * tw, pan.y + row * th, tw, th);
          }
        }
      }
    }
  }, [tiles, palettes, pan, zoom, paletteImages, paintMode, comp.map_cols, comp.map_rows, comp.tile_width, comp.tile_height, canvasSize]);

  const canvasToTile = useCallback((clientX: number, clientY: number) => {
    const canvas = canvasRef.current;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    const mx = (clientX - rect.left) * (canvas.width / rect.width);
    const my = (clientY - rect.top) * (canvas.height / rect.height);
    const tw = comp.tile_width * zoom;
    const th = comp.tile_height * zoom;
    const col = Math.floor((mx - pan.x) / tw);
    const row = Math.floor((my - pan.y) / th);
    if (col < 0 || row < 0 || col >= comp.map_cols || row >= comp.map_rows) return null;
    return { col, row, idx: row * comp.map_cols + col };
  }, [pan, zoom, comp.tile_width, comp.tile_height, comp.map_cols, comp.map_rows]);

  const paintAt = useCallback((col: number, row: number) => {
    if (paintMode === "collision") return;
    const idx = row * comp.map_cols + col;
    const pal = palettes[selectedPaletteIdx];
    const val = paintMode === "erase" ? 0 : (pal ? encodeTile(selectedPaletteIdx + 1, selectedTileIdx) : 0);
    if (paintMode !== "erase" && !pal) return;
    setTiles(prev => {
      if (idx < 0 || idx >= prev.length || prev[idx] === val) return prev;
      const next = [...prev];
      next[idx] = val;
      scheduleSave(next, null);
      return next;
    });
  }, [paintMode, selectedPaletteIdx, selectedTileIdx, palettes, comp.map_cols, scheduleSave]);

  const floodFill = useCallback((startIdx: number) => {
    const pal = palettes[selectedPaletteIdx];
    const fillId = paintMode === "erase" ? 0 : (pal ? encodeTile(selectedPaletteIdx + 1, selectedTileIdx) : 0);
    setTiles(prev => {
      const targetId = prev[startIdx];
      if (targetId === fillId) return prev;
      const next = [...prev];
      const cols = comp.map_cols;
      const stack = [startIdx];
      const visited = new Set<number>();
      while (stack.length > 0) {
        const i = stack.pop()!;
        if (visited.has(i) || i < 0 || i >= next.length || next[i] !== targetId) continue;
        visited.add(i);
        next[i] = fillId;
        const c = i % cols;
        const r = Math.floor(i / cols);
        if (c > 0) stack.push(i - 1);
        if (c < cols - 1) stack.push(i + 1);
        if (r > 0) stack.push(i - cols);
        if (r < comp.map_rows - 1) stack.push(i + cols);
      }
      scheduleSave(next, null);
      return next;
    });
  }, [paintMode, selectedPaletteIdx, selectedTileIdx, palettes, comp.map_cols, comp.map_rows, scheduleSave]);

  const handleCanvasPointerDown = useCallback((e: React.PointerEvent<HTMLCanvasElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    if (e.button === 1 || spaceHeld.current) {
      panStartRef.current = { mx: e.clientX, my: e.clientY, px: pan.x, py: pan.y };
      return;
    }
    if (e.button !== 0) return;
    isPainting.current = true;
    const tile = canvasToTile(e.clientX, e.clientY);
    if (tile) paintAt(tile.col, tile.row);
  }, [pan, canvasToTile, paintAt]);

  const handleCanvasPointerMove = useCallback((e: React.PointerEvent<HTMLCanvasElement>) => {
    if (panStartRef.current) {
      const dx = e.clientX - panStartRef.current.mx;
      const dy = e.clientY - panStartRef.current.my;
      setPan({ x: panStartRef.current.px + dx, y: panStartRef.current.py + dy });
      return;
    }
    if (!isPainting.current) return;
    const tile = canvasToTile(e.clientX, e.clientY);
    if (tile) paintAt(tile.col, tile.row);
  }, [canvasToTile, paintAt]);

  const handleCanvasPointerUp = useCallback(() => {
    isPainting.current = false;
    panStartRef.current = null;
  }, []);

  const handleCanvasClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (e.shiftKey && paintMode !== "collision") {
      const tile = canvasToTile(e.clientX, e.clientY);
      if (tile) floodFill(tile.idx);
    }
  }, [paintMode, canvasToTile, floodFill]);

  const handleWheel = useCallback((e: React.WheelEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const mx = (e.clientX - rect.left) * (canvas.width / rect.width);
    const my = (e.clientY - rect.top) * (canvas.height / rect.height);
    const factor = e.deltaY < 0 ? 1.12 : 0.89;
    const newZoom = Math.max(0.15, Math.min(10, zoom * factor));
    setPan(prev => ({
      x: mx - (mx - prev.x) * (newZoom / zoom),
      y: my - (my - prev.y) * (newZoom / zoom),
    }));
    setZoom(newZoom);
  }, [zoom]);

  const toggleSolidTile = useCallback((tileIdx: number) => {
    setPalettes(prev => {
      const next = prev.map((p, i) => {
        if (i !== selectedPaletteIdx) return p;
        const solid = p.solid_tiles.includes(tileIdx)
          ? p.solid_tiles.filter(id => id !== tileIdx)
          : [...p.solid_tiles, tileIdx];
        return { ...p, solid_tiles: solid };
      });
      scheduleSave(null, next);
      return next;
    });
  }, [selectedPaletteIdx, scheduleSave]);

  const handleTilesetClick = useCallback((tileIdx: number) => {
    if (paintMode === "collision") {
      toggleSolidTile(tileIdx);
    } else {
      setSelectedTileIdx(tileIdx);
      if (paintMode === "erase") setPaintMode("draw");
    }
  }, [paintMode, toggleSolidTile]);

  const selectedPalette = palettes[selectedPaletteIdx];
  const selectedImg = selectedPalette ? paletteImages.get(selectedPalette.texture_path) : undefined;
  const tsC = Math.max(1, selectedPalette?.tileset_cols ?? 1);
  const tsR = Math.max(1, selectedPalette?.tileset_rows ?? 1);
  const TILESET_CELL = Math.max(20, Math.min(48, Math.floor(240 / tsC)));

  const modeBtn = (mode: PaintMode): React.CSSProperties => ({
    background: paintMode === mode ? (mode === "collision" ? "rgba(220,50,50,0.8)" : "var(--amber)") : "none",
    border: `1px solid ${paintMode === mode ? "transparent" : "var(--rule-2)"}`,
    color: paintMode === mode ? "var(--paper)" : "var(--ink-3)",
    fontFamily: "var(--font-mono)", fontSize: "10px", padding: "3px 10px", cursor: "pointer",
  });

  return (
    <div style={{ position: "fixed", inset: 0, zIndex: 9000, background: "rgba(0,0,0,0.72)", display: "flex", alignItems: "center", justifyContent: "center" }}>
      <div style={{ background: "var(--paper)", border: "1px solid var(--rule-2)", display: "flex", flexDirection: "column", width: "min(96vw, 1200px)", height: "min(92vh, 820px)", overflow: "hidden" }}>

        {/* Header */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "8px 16px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-3)" }}>
            TILE PAINTER · {comp.map_cols}×{comp.map_rows}
            {palettes.length > 0 && ` · ${palettes.length} palette${palettes.length !== 1 ? "s" : ""}`}
          </span>
          <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
            <button onClick={() => setPaintMode("draw")} style={modeBtn("draw")}>Draw</button>
            <button onClick={() => setPaintMode("erase")} style={modeBtn("erase")}>Erase</button>
            <button onClick={() => setPaintMode("collision")} style={modeBtn("collision")} title="Mark tiles solid for physics">Collision</button>
            <div style={{ width: 1, height: 16, background: "var(--rule-2)", margin: "0 2px" }} />
            <button onClick={() => { const c = new Array(comp.map_cols * comp.map_rows).fill(0); setTiles(c); flush(c, null); }}
              style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "3px 10px", cursor: "pointer" }}>Clear</button>
            <button onClick={onClose}
              style={{ background: "none", border: "none", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "18px", cursor: "pointer", padding: "0 4px" }}>×</button>
          </div>
        </div>

        {/* Mode hint bar */}
        {paintMode === "collision" ? (
          <div style={{ padding: "4px 16px", background: "rgba(200,50,50,0.12)", borderBottom: "1px solid rgba(200,50,50,0.3)", flexShrink: 0 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "rgba(220,100,100,0.9)" }}>
              Click tiles in the palette to toggle solid · Shift+click map to flood fill
            </span>
          </div>
        ) : (
          <div style={{ padding: "4px 16px", background: "rgba(0,0,0,0.1)", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
              Scroll to zoom · Middle-drag or Space+drag to pan · Shift+click to flood fill
              {selectedPalette && paintMode !== "erase" && ` · Palette "${selectedPalette.name || `P${selectedPaletteIdx + 1}`}" · Tile #${selectedTileIdx}`}
            </span>
          </div>
        )}

        <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
          {/* Left: palette tabs + tileset picker */}
          <div style={{ width: "280px", flexShrink: 0, borderRight: "1px solid var(--rule)", display: "flex", flexDirection: "column", overflow: "hidden" }}>

            {/* Palette tabs */}
            {palettes.length > 1 && (
              <div style={{ display: "flex", flexWrap: "wrap", gap: "2px", padding: "6px 8px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
                {palettes.map((p, i) => (
                  <button key={i} onClick={() => { setSelectedPaletteIdx(i); setSelectedTileIdx(0); }} style={{
                    fontFamily: "var(--font-mono)", fontSize: "10px", padding: "2px 8px", cursor: "pointer",
                    background: i === selectedPaletteIdx ? "var(--amber)" : "none",
                    border: `1px solid ${i === selectedPaletteIdx ? "transparent" : "var(--rule-2)"}`,
                    color: i === selectedPaletteIdx ? "var(--paper)" : "var(--ink-3)",
                  }}>{p.name || `P${i + 1}`}</button>
                ))}
              </div>
            )}

            {/* Tileset label */}
            <div style={{ padding: "5px 12px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
                {selectedPalette ? `${selectedPalette.name || "Palette"} · ${tsC}×${tsR}${paintMode === "collision" ? " · click = toggle solid" : ""}` : "No palettes — add one in Inspector"}
              </span>
            </div>

            {/* Tileset grid */}
            <div style={{ overflow: "auto", flex: 1, padding: "8px" }}>
              {selectedPalette && selectedImg ? (
                <div style={{ display: "grid", gridTemplateColumns: `repeat(${tsC}, ${TILESET_CELL}px)`, gap: 0, width: `${tsC * TILESET_CELL}px` }}>
                  {Array.from({ length: tsC * tsR }, (_, i) => {
                    const m = selectedPalette.margin ?? 0;
                    const s = selectedPalette.spacing ?? 0;
                    const iw = selectedImg.naturalWidth;
                    const ih = selectedImg.naturalHeight;
                    let bgX: number, bgY: number;
                    const bgW = TILESET_CELL * tsC;
                    const bgH = TILESET_CELL * tsR;
                    if (m === 0 && s === 0) {
                      bgX = -(i % tsC) * TILESET_CELL;
                      bgY = -Math.floor(i / tsC) * TILESET_CELL;
                    } else {
                      const cellW = (iw - 2 * m - s * (tsC - 1)) / tsC;
                      const cellH = (ih - 2 * m - s * (tsR - 1)) / tsR;
                      const px = m + (i % tsC) * (cellW + s);
                      const py = m + Math.floor(i / tsC) * (cellH + s);
                      bgX = -(px / iw) * bgW;
                      bgY = -(py / ih) * bgH;
                    }
                    const isSelected = selectedTileIdx === i && paintMode !== "collision";
                    const isSolid = selectedPalette.solid_tiles.includes(i);
                    return (
                      <div key={i} onClick={() => handleTilesetClick(i)} title={`Tile #${i}${isSolid ? " (solid)" : ""}`} style={{
                        width: TILESET_CELL, height: TILESET_CELL, position: "relative", cursor: "pointer", boxSizing: "border-box",
                        backgroundImage: `url(${resolveTextureUrl(selectedPalette.texture_path)})`,
                        backgroundPosition: `${bgX}px ${bgY}px`,
                        backgroundSize: `${bgW}px ${bgH}px`,
                        outline: isSelected ? "2px solid var(--amber)" : isSolid ? "2px solid rgba(220,80,80,0.8)" : "1px solid rgba(255,255,255,0.05)",
                        outlineOffset: "-2px",
                        imageRendering: "pixelated",
                      }}>
                        {isSolid && <div style={{ position: "absolute", inset: 0, background: "rgba(220,60,60,0.25)", pointerEvents: "none" }} />}
                      </div>
                    );
                  })}
                </div>
              ) : (
                <div style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "12px 0" }}>
                  {palettes.length === 0
                    ? "Add a palette in Inspector"
                    : selectedPalette && !selectedPalette.texture_path
                      ? "No texture set for this palette"
                      : "Loading…"}
                </div>
              )}
            </div>
          </div>

          {/* Right: canvas map */}
          <div ref={canvasContainerRef} style={{ flex: 1, overflow: "hidden", position: "relative" }}>
            <canvas
              ref={canvasRef}
              width={canvasSize.w}
              height={canvasSize.h}
              style={{ width: "100%", height: "100%", display: "block", cursor: "crosshair" }}
              onPointerDown={handleCanvasPointerDown}
              onPointerMove={handleCanvasPointerMove}
              onPointerUp={handleCanvasPointerUp}
              onClick={handleCanvasClick}
              onWheel={handleWheel}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
