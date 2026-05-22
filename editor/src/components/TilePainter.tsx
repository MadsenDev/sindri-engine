import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

interface TileLayer {
  name: string;
  tiles: number[];
  visible: boolean;
  opacity: number;
  z_index: number;
}

interface TilemapComp {
  palettes: TilePalette[];
  tile_width: number;
  tile_height: number;
  map_cols: number;
  map_rows: number;
  layers: TileLayer[];
  tint: [number, number, number, number];
}

interface Props {
  comp: TilemapComp;
  entityId: number;
  componentIdx: number;
  onClose: () => void;
  projectPath?: string | null;
  onSceneChange?: () => void;
}

interface PrefabInfo { name: string; path: string; }

type PaintMode = "draw" | "erase" | "collision" | "stamp";

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
    srcW = iw / cols; srcH = ih / rows;
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

export default function TilePainter({ comp, entityId, componentIdx, onClose, projectPath, onSceneChange }: Props) {
  const initLayers = () =>
    comp.layers.length > 0
      ? comp.layers.map(l => ({ ...l, tiles: [...l.tiles] }))
      : [{ name: "Ground", tiles: new Array(comp.map_cols * comp.map_rows).fill(0), visible: true, opacity: 1.0, z_index: 0 }];

  const [layers, setLayers] = useState<TileLayer[]>(initLayers);
  const [palettes, setPalettes] = useState<TilePalette[]>(() =>
    comp.palettes.map(p => ({ ...p, solid_tiles: [...(p.solid_tiles ?? [])] }))
  );
  const [activeLayerIdx, setActiveLayerIdx] = useState(0);
  const [selectedPaletteIdx, setSelectedPaletteIdx] = useState(0);
  const [selectedTileIdx, setSelectedTileIdx] = useState(0);
  const [paintMode, setPaintMode] = useState<PaintMode>("draw");
  const [paletteImages, setPaletteImages] = useState<Map<string, HTMLImageElement>>(new Map());
  const [pan, setPan] = useState({ x: 16, y: 16 });
  const [zoom, setZoom] = useState(1.0);
  const [canvasSize, setCanvasSize] = useState({ w: 800, h: 600 });
  const [editingLayerName, setEditingLayerName] = useState<number | null>(null);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const canvasContainerRef = useRef<HTMLDivElement>(null);
  const isPainting = useRef(false);
  const panStartRef = useRef<{ mx: number; my: number; px: number; py: number } | null>(null);
  const spaceHeld = useRef(false);
  const loadedPathsRef = useRef<Set<string>>(new Set());
  const pendingLayersRef = useRef<TileLayer[] | null>(null);
  const pendingPalettesRef = useRef<TilePalette[] | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [prefabs, setPrefabs] = useState<PrefabInfo[]>([]);
  const [selectedPrefabPath, setSelectedPrefabPath] = useState<string | null>(null);

  useEffect(() => {
    const container = canvasContainerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;
    const ro = new ResizeObserver(entries => {
      for (const entry of entries) {
        const w = Math.floor(entry.contentRect.width);
        const h = Math.floor(entry.contentRect.height);
        canvas.width = w; canvas.height = h;
        setCanvasSize({ w, h });
      }
    });
    ro.observe(container);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    palettes.forEach(p => {
      if (!p.texture_path || loadedPathsRef.current.has(p.texture_path)) return;
      loadedPathsRef.current.add(p.texture_path);
      const img = new Image();
      img.onload = () => setPaletteImages(prev => new Map(prev).set(p.texture_path, img));
      img.src = resolveTextureUrl(p.texture_path);
    });
  }, [palettes]);

  useEffect(() => {
    const down = (e: KeyboardEvent) => { if (e.code === "Space" && e.target === document.body) { spaceHeld.current = true; e.preventDefault(); } };
    const up = (e: KeyboardEvent) => { if (e.code === "Space") spaceHeld.current = false; };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => { window.removeEventListener("keydown", down); window.removeEventListener("keyup", up); };
  }, []);

  useEffect(() => {
    if (paintMode !== "stamp" || !projectPath) return;
    invoke<PrefabInfo[]>("list_prefabs", { projectPath }).then(setPrefabs).catch(() => {});
  }, [paintMode, projectPath]);

  const flush = useCallback(async (newLayers: TileLayer[] | null, newPalettes: TilePalette[] | null) => {
    const data: Record<string, unknown> = {};
    if (newLayers !== null) data.layers = newLayers;
    if (newPalettes !== null) data.palettes = newPalettes;
    if (Object.keys(data).length === 0) return;
    try { await invoke("patch_component", { entityId, componentIdx, data }); }
    catch (e) { console.error("tile save failed:", e); }
  }, [entityId, componentIdx]);

  const scheduleSave = useCallback((newLayers: TileLayer[] | null, newPalettes: TilePalette[] | null) => {
    if (newLayers !== null) pendingLayersRef.current = newLayers;
    if (newPalettes !== null) pendingPalettesRef.current = newPalettes;
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      flush(pendingLayersRef.current, pendingPalettesRef.current);
      pendingLayersRef.current = null; pendingPalettesRef.current = null;
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

    // Grid
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

    ctx.imageSmoothingEnabled = false;

    // Render all layers bottom-to-top
    for (let li = 0; li < layers.length; li++) {
      const layer = layers[li];
      if (!layer.visible) continue;
      const isActive = li === activeLayerIdx;
      const baseAlpha = layer.opacity * (isActive ? 1.0 : 0.45);

      for (let row = 0; row < comp.map_rows; row++) {
        for (let col = 0; col < comp.map_cols; col++) {
          const cell = layer.tiles[row * comp.map_cols + col] ?? 0;
          if (cell === 0) continue;
          const { paletteId, tileIdx } = decodeTile(cell);
          if (paletteId === 0) continue;
          const pal = palettes[paletteId - 1];
          if (!pal) continue;
          const img = paletteImages.get(pal.texture_path);
          if (!img) continue;
          ctx.globalAlpha = baseAlpha;
          drawTileOnCanvas(ctx, img, tileIdx, pal, pan.x + col * tw, pan.y + row * th, tw, th);
        }
      }
    }
    ctx.globalAlpha = 1;

    // Solid overlays on active layer in collision mode
    if (paintMode === "collision") {
      const activeLayer = layers[activeLayerIdx];
      if (activeLayer) {
        ctx.fillStyle = "rgba(220,60,60,0.35)";
        for (let row = 0; row < comp.map_rows; row++) {
          for (let col = 0; col < comp.map_cols; col++) {
            const cell = activeLayer.tiles[row * comp.map_cols + col] ?? 0;
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
    }
  }, [layers, palettes, pan, zoom, paletteImages, paintMode, activeLayerIdx, comp.map_cols, comp.map_rows, comp.tile_width, comp.tile_height, canvasSize]);

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
    setLayers(prev => {
      const layer = prev[activeLayerIdx];
      if (!layer || idx < 0 || idx >= layer.tiles.length || layer.tiles[idx] === val) return prev;
      const newLayer = { ...layer, tiles: [...layer.tiles] };
      newLayer.tiles[idx] = val;
      const next = prev.map((l, i) => i === activeLayerIdx ? newLayer : l);
      scheduleSave(next, null);
      return next;
    });
  }, [paintMode, selectedPaletteIdx, selectedTileIdx, palettes, activeLayerIdx, comp.map_cols, scheduleSave]);

  const floodFill = useCallback((startIdx: number) => {
    const pal = palettes[selectedPaletteIdx];
    const fillId = paintMode === "erase" ? 0 : (pal ? encodeTile(selectedPaletteIdx + 1, selectedTileIdx) : 0);
    setLayers(prev => {
      const layer = prev[activeLayerIdx];
      if (!layer) return prev;
      const targetId = layer.tiles[startIdx];
      if (targetId === fillId) return prev;
      const newTiles = [...layer.tiles];
      const cols = comp.map_cols;
      const stack = [startIdx];
      const visited = new Set<number>();
      while (stack.length > 0) {
        const i = stack.pop()!;
        if (visited.has(i) || i < 0 || i >= newTiles.length || newTiles[i] !== targetId) continue;
        visited.add(i);
        newTiles[i] = fillId;
        const c = i % cols; const r = Math.floor(i / cols);
        if (c > 0) stack.push(i - 1);
        if (c < cols - 1) stack.push(i + 1);
        if (r > 0) stack.push(i - cols);
        if (r < comp.map_rows - 1) stack.push(i + cols);
      }
      const next = prev.map((l, i) => i === activeLayerIdx ? { ...l, tiles: newTiles } : l);
      scheduleSave(next, null);
      return next;
    });
  }, [paintMode, selectedPaletteIdx, selectedTileIdx, palettes, activeLayerIdx, comp.map_cols, comp.map_rows, scheduleSave]);

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
      setPan({ x: panStartRef.current.px + e.clientX - panStartRef.current.mx, y: panStartRef.current.py + e.clientY - panStartRef.current.my });
      return;
    }
    if (!isPainting.current) return;
    const tile = canvasToTile(e.clientX, e.clientY);
    if (tile) paintAt(tile.col, tile.row);
  }, [canvasToTile, paintAt]);

  const handleCanvasPointerUp = useCallback(() => { isPainting.current = false; panStartRef.current = null; }, []);

  const handleStampClick = useCallback(async (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!selectedPrefabPath || !projectPath) return;
    const tile = canvasToTile(e.clientX, e.clientY);
    if (!tile) return;
    // World position = tile center relative to tilemap entity origin (0,0)
    const wx = (tile.col + 0.5) * comp.tile_width;
    const wy = (tile.row + 0.5) * comp.tile_height;
    try {
      const newId = await invoke<number>("instantiate_prefab", { projectPath, prefabPath: selectedPrefabPath });
      await invoke("patch_transform", { entityId: newId, x: wx, y: wy, scaleX: 1, scaleY: 1, rotation: 0 });
      onSceneChange?.();
    } catch (e) { console.error("stamp prefab failed:", e); }
  }, [selectedPrefabPath, projectPath, canvasToTile, comp.tile_width, comp.tile_height, onSceneChange]);

  const handleCanvasClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (paintMode === "stamp") { handleStampClick(e); return; }
    if (e.shiftKey && paintMode !== "collision") {
      const tile = canvasToTile(e.clientX, e.clientY);
      if (tile) floodFill(tile.idx);
    }
  }, [paintMode, canvasToTile, floodFill, handleStampClick]);

  const handleWheel = useCallback((e: React.WheelEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const mx = (e.clientX - rect.left) * (canvas.width / rect.width);
    const my = (e.clientY - rect.top) * (canvas.height / rect.height);
    const factor = e.deltaY < 0 ? 1.12 : 0.89;
    const newZoom = Math.max(0.15, Math.min(10, zoom * factor));
    setPan(prev => ({ x: mx - (mx - prev.x) * (newZoom / zoom), y: my - (my - prev.y) * (newZoom / zoom) }));
    setZoom(newZoom);
  }, [zoom]);

  const toggleSolidTile = useCallback((tileIdx: number) => {
    setPalettes(prev => {
      const next = prev.map((p, i) => {
        if (i !== selectedPaletteIdx) return p;
        const solid = p.solid_tiles.includes(tileIdx) ? p.solid_tiles.filter(id => id !== tileIdx) : [...p.solid_tiles, tileIdx];
        return { ...p, solid_tiles: solid };
      });
      scheduleSave(null, next);
      return next;
    });
  }, [selectedPaletteIdx, scheduleSave]);

  const addLayer = () => {
    const tileCount = comp.map_cols * comp.map_rows;
    const newLayer: TileLayer = { name: `Layer ${layers.length + 1}`, tiles: new Array(tileCount).fill(0), visible: true, opacity: 1.0, z_index: 0 };
    const next = [...layers, newLayer];
    setLayers(next);
    setActiveLayerIdx(next.length - 1);
    scheduleSave(next, null);
  };

  const removeLayer = (i: number) => {
    if (layers.length <= 1) return;
    const next = layers.filter((_, idx) => idx !== i);
    setLayers(next);
    setActiveLayerIdx(Math.min(activeLayerIdx, next.length - 1));
    scheduleSave(next, null);
  };

  const toggleLayerVisible = (i: number) => {
    const next = layers.map((l, idx) => idx === i ? { ...l, visible: !l.visible } : l);
    setLayers(next);
    scheduleSave(next, null);
  };

  const renameLayer = (i: number, name: string) => {
    const next = layers.map((l, idx) => idx === i ? { ...l, name } : l);
    setLayers(next);
    setEditingLayerName(null);
    scheduleSave(next, null);
  };

  const moveLayer = (i: number, dir: -1 | 1) => {
    const j = i + dir;
    if (j < 0 || j >= layers.length) return;
    const next = [...layers];
    [next[i], next[j]] = [next[j], next[i]];
    setLayers(next);
    setActiveLayerIdx(j);
    scheduleSave(next, null);
  };

  const setLayerZIndex = (i: number, z: number) => {
    const next = layers.map((l, idx) => idx === i ? { ...l, z_index: z } : l);
    setLayers(next);
    scheduleSave(next, null);
  };

  const selectedPalette = palettes[selectedPaletteIdx];
  const selectedImg = selectedPalette ? paletteImages.get(selectedPalette.texture_path) : undefined;
  const tsC = Math.max(1, selectedPalette?.tileset_cols ?? 1);
  const tsR = Math.max(1, selectedPalette?.tileset_rows ?? 1);
  const TILESET_CELL = Math.max(18, Math.min(48, Math.floor(240 / tsC)));

  const modeBtn = (mode: PaintMode): React.CSSProperties => ({
    background: paintMode === mode ? (mode === "collision" ? "rgba(220,50,50,0.8)" : mode === "stamp" ? "rgba(180,140,60,0.8)" : "var(--amber)") : "none",
    border: `1px solid ${paintMode === mode ? "transparent" : "var(--rule-2)"}`,
    color: paintMode === mode ? "var(--paper)" : "var(--ink-3)",
    fontFamily: "var(--font-mono)", fontSize: "10px", padding: "3px 8px", cursor: "pointer",
  });

  const iconBtn: React.CSSProperties = { background: "none", border: "none", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "12px", cursor: "pointer", padding: "0 3px" };

  return (
    <div style={{ position: "fixed", inset: 0, zIndex: 9000, background: "rgba(0,0,0,0.72)", display: "flex", alignItems: "center", justifyContent: "center" }}>
      <div style={{ background: "var(--paper)", border: "1px solid var(--rule-2)", display: "flex", flexDirection: "column", width: "min(96vw, 1200px)", height: "min(92vh, 820px)", overflow: "hidden" }}>

        {/* Header */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "8px 16px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-3)" }}>
            TILE PAINTER · {comp.map_cols}×{comp.map_rows} · {layers.length} layer{layers.length !== 1 ? "s" : ""}
          </span>
          <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
            <button onClick={() => setPaintMode("draw")} style={modeBtn("draw")}>Draw</button>
            <button onClick={() => setPaintMode("erase")} style={modeBtn("erase")}>Erase</button>
            <button onClick={() => setPaintMode("collision")} style={modeBtn("collision")}>Collision</button>
            {projectPath && <button onClick={() => setPaintMode("stamp")} style={modeBtn("stamp")}>Stamp</button>}
            <div style={{ width: 1, height: 16, background: "var(--rule-2)", margin: "0 2px" }} />
            <button onClick={() => {
              const tileCount = comp.map_cols * comp.map_rows;
              const next = layers.map(l => ({ ...l, tiles: new Array(tileCount).fill(0) }));
              setLayers(next); flush(next, null);
            }} style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "3px 8px", cursor: "pointer" }}>Clear All</button>
            <button onClick={() => {
              const layer = layers[activeLayerIdx];
              if (!layer) return;
              const tileCount = comp.map_cols * comp.map_rows;
              const next = layers.map((l, i) => i === activeLayerIdx ? { ...l, tiles: new Array(tileCount).fill(0) } : l);
              setLayers(next); flush(next, null);
            }} style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "3px 8px", cursor: "pointer" }}>Clear Layer</button>
            <button onClick={onClose} style={{ background: "none", border: "none", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "18px", cursor: "pointer", padding: "0 4px" }}>×</button>
          </div>
        </div>

        {/* Hint bar */}
        {paintMode === "collision" ? (
          <div style={{ padding: "4px 16px", background: "rgba(200,50,50,0.12)", borderBottom: "1px solid rgba(200,50,50,0.3)", flexShrink: 0 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "rgba(220,100,100,0.9)" }}>Click palette tiles to toggle solid · Shift+click map to flood fill</span>
          </div>
        ) : (
          <div style={{ padding: "4px 16px", background: "rgba(0,0,0,0.1)", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
              Scroll=zoom · Middle/Space+drag=pan · Shift+click=flood fill
              {selectedPalette && paintMode !== "erase" && ` · "${selectedPalette.name || `P${selectedPaletteIdx + 1}`}" tile #${selectedTileIdx}`}
              {` · Active: "${layers[activeLayerIdx]?.name ?? "?"}"`}
            </span>
          </div>
        )}

        <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
          {/* Left panel: palette + layers */}
          <div style={{ width: "240px", flexShrink: 0, borderRight: "1px solid var(--rule)", display: "flex", flexDirection: "column", overflow: "hidden" }}>

            {/* Stamp mode — prefab list */}
            {paintMode === "stamp" && (
              <>
                <div style={{ padding: "5px 10px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>Prefabs · click map to stamp</span>
                </div>
                <div style={{ overflow: "auto", flex: 1, padding: "4px 0" }}>
                  {prefabs.length === 0 ? (
                    <div style={{ padding: "10px", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px" }}>
                      No prefabs found in prefabs/
                    </div>
                  ) : (
                    prefabs.map(p => (
                      <div
                        key={p.path}
                        onClick={() => setSelectedPrefabPath(prev => prev === p.path ? null : p.path)}
                        style={{
                          display: "flex", alignItems: "center", gap: "7px",
                          padding: "5px 10px", cursor: "pointer",
                          background: selectedPrefabPath === p.path ? "var(--amber)" : "none",
                        }}
                      >
                        <span style={{ fontSize: "9px", color: selectedPrefabPath === p.path ? "var(--paper)" : "var(--amber)" }}>◆</span>
                        <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: selectedPrefabPath === p.path ? "var(--paper)" : "var(--ink)" }}>{p.name}</span>
                      </div>
                    ))
                  )}
                </div>
              </>
            )}

            {/* Palette tabs */}
            {paintMode !== "stamp" && palettes.length > 1 && (
              <div style={{ display: "flex", flexWrap: "wrap", gap: "2px", padding: "5px 8px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
                {palettes.map((p, i) => (
                  <button key={i} onClick={() => { setSelectedPaletteIdx(i); setSelectedTileIdx(0); }} style={{
                    fontFamily: "var(--font-mono)", fontSize: "10px", padding: "2px 7px", cursor: "pointer",
                    background: i === selectedPaletteIdx ? "var(--amber)" : "none",
                    border: `1px solid ${i === selectedPaletteIdx ? "transparent" : "var(--rule-2)"}`,
                    color: i === selectedPaletteIdx ? "var(--paper)" : "var(--ink-3)",
                  }}>{p.name || `P${i + 1}`}</button>
                ))}
              </div>
            )}

            {/* Tileset label */}
            {paintMode !== "stamp" && (
            <div style={{ padding: "5px 10px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
                {selectedPalette ? `${selectedPalette.name || "Palette"} · ${tsC}×${tsR}${paintMode === "collision" ? " · click=solid" : ""}` : "No palettes — add in Inspector"}
              </span>
            </div>
            )}

            {/* Tileset grid */}
            {paintMode !== "stamp" && <div style={{ overflow: "auto", flex: 1, padding: "6px" }}>
              {selectedPalette && selectedImg ? (
                <div style={{ display: "grid", gridTemplateColumns: `repeat(${tsC}, ${TILESET_CELL}px)`, gap: 0, width: `${tsC * TILESET_CELL}px` }}>
                  {Array.from({ length: tsC * tsR }, (_, i) => {
                    const m = selectedPalette.margin ?? 0;
                    const s = selectedPalette.spacing ?? 0;
                    const iw = selectedImg.naturalWidth; const ih = selectedImg.naturalHeight;
                    let bgX: number, bgY: number;
                    const bgW = TILESET_CELL * tsC; const bgH = TILESET_CELL * tsR;
                    if (m === 0 && s === 0) {
                      bgX = -(i % tsC) * TILESET_CELL; bgY = -Math.floor(i / tsC) * TILESET_CELL;
                    } else {
                      const cw = (iw - 2 * m - s * (tsC - 1)) / tsC; const ch = (ih - 2 * m - s * (tsR - 1)) / tsR;
                      bgX = -(m + (i % tsC) * (cw + s)) / iw * bgW; bgY = -(m + Math.floor(i / tsC) * (ch + s)) / ih * bgH;
                    }
                    const isSelected = selectedTileIdx === i && paintMode !== "collision";
                    const isSolid = selectedPalette.solid_tiles.includes(i);
                    return (
                      <div key={i} onClick={() => paintMode === "collision" ? toggleSolidTile(i) : (setSelectedTileIdx(i), paintMode === "erase" && setPaintMode("draw"))}
                        title={`Tile #${i}${isSolid ? " (solid)" : ""}`}
                        style={{ width: TILESET_CELL, height: TILESET_CELL, position: "relative", cursor: "pointer", boxSizing: "border-box",
                          backgroundImage: `url(${resolveTextureUrl(selectedPalette.texture_path)})`,
                          backgroundPosition: `${bgX}px ${bgY}px`, backgroundSize: `${bgW}px ${bgH}px`,
                          outline: isSelected ? "2px solid var(--amber)" : isSolid ? "2px solid rgba(220,80,80,0.8)" : "1px solid rgba(255,255,255,0.05)",
                          outlineOffset: "-2px", imageRendering: "pixelated" }}>
                        {isSolid && <div style={{ position: "absolute", inset: 0, background: "rgba(220,60,60,0.25)", pointerEvents: "none" }} />}
                      </div>
                    );
                  })}
                </div>
              ) : (
                <div style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "10px 0" }}>
                  {palettes.length === 0 ? "Add a palette in Inspector" : "Loading…"}
                </div>
              )}
            </div>}

            {/* Layer list */}
            <div style={{ borderTop: "1px solid var(--rule)", flexShrink: 0, maxHeight: "220px", display: "flex", flexDirection: "column" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "5px 10px", borderBottom: "1px solid var(--rule)" }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>LAYERS</span>
                <button onClick={addLayer} style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "1px 7px", cursor: "pointer" }}>+ Add</button>
              </div>
              <div style={{ overflow: "auto", flex: 1 }}>
                {[...layers].reverse().map((layer, ri) => {
                  const i = layers.length - 1 - ri;
                  const isActive = i === activeLayerIdx;
                  return (
                    <div key={i} onClick={() => setActiveLayerIdx(i)} style={{
                      display: "flex", alignItems: "center", gap: "4px",
                      padding: "4px 8px", cursor: "pointer",
                      background: isActive ? "rgba(220,160,20,0.15)" : "transparent",
                      borderLeft: isActive ? "2px solid var(--amber)" : "2px solid transparent",
                    }}>
                      <button onClick={e => { e.stopPropagation(); toggleLayerVisible(i); }} style={{ ...iconBtn, opacity: layer.visible ? 1 : 0.35 }} title={layer.visible ? "Hide" : "Show"}>
                        {layer.visible ? "◉" : "○"}
                      </button>
                      {editingLayerName === i ? (
                        <input autoFocus defaultValue={layer.name}
                          onBlur={e => renameLayer(i, e.currentTarget.value || layer.name)}
                          onKeyDown={e => { if (e.key === "Enter") renameLayer(i, e.currentTarget.value || layer.name); e.stopPropagation(); }}
                          onClick={e => e.stopPropagation()}
                          style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: "10px", background: "var(--paper-2)", border: "1px solid var(--amber)", color: "var(--ink)", padding: "1px 4px", outline: "none" }}
                        />
                      ) : (
                        <span onDoubleClick={e => { e.stopPropagation(); setEditingLayerName(i); }} style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: "10px", color: isActive ? "var(--ink)" : "var(--ink-3)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                          {layer.name || `Layer ${i + 1}`}
                        </span>
                      )}
                      <input
                        type="number"
                        title="z-index"
                        defaultValue={layer.z_index ?? 0}
                        onClick={e => e.stopPropagation()}
                        onBlur={e => setLayerZIndex(i, parseInt(e.currentTarget.value) || 0)}
                        onKeyDown={e => { if (e.key === "Enter") { setLayerZIndex(i, parseInt(e.currentTarget.value) || 0); e.stopPropagation(); } e.stopPropagation(); }}
                        style={{ width: "36px", fontFamily: "var(--font-mono)", fontSize: "9px", background: "var(--paper-2)", border: "1px solid var(--rule-2)", color: "var(--ink-3)", padding: "1px 3px", textAlign: "center", outline: "none" }}
                      />
                      <button onClick={e => { e.stopPropagation(); moveLayer(i, 1); }} style={iconBtn} title="Move up">↑</button>
                      <button onClick={e => { e.stopPropagation(); moveLayer(i, -1); }} style={iconBtn} title="Move down">↓</button>
                      <button onClick={e => { e.stopPropagation(); removeLayer(i); }} disabled={layers.length <= 1} style={{ ...iconBtn, opacity: layers.length <= 1 ? 0.2 : 0.6 }} title="Delete layer">×</button>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>

          {/* Canvas map */}
          <div ref={canvasContainerRef} style={{ flex: 1, overflow: "hidden", position: "relative" }}>
            <canvas ref={canvasRef} width={canvasSize.w} height={canvasSize.h}
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
