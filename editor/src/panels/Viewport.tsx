import { useRef, useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ActiveTool, Scene, Entity, TilemapEdit } from "../App";

export interface ColliderChange {
  entityId: number;
  componentIdx: number;
  before: { width: number; height: number; offset_x: number; offset_y: number };
  after: { width: number; height: number; offset_x: number; offset_y: number };
}

interface Props {
  scene: Scene | null;
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  activeTool: ActiveTool;
  onTransformCommit: (change: TransformChange) => void;
  onColliderCommit: (change: ColliderChange) => void;
  engineReady: boolean;
  isPlaying: boolean;
  resolution?: string;
  tilemapEdit?: TilemapEdit | null;
  onTilemapEditChange?: (e: TilemapEdit) => void;
  onSceneChange?: () => void;
  projectPath?: string | null;
}

interface Camera {
  x: number;   // world-space center
  y: number;
  zoom: number; // pixels per world unit
}

const BG = "#0d1117";
const GRID_MINOR = "rgba(230,225,212,0.04)";
const GRID_MAJOR = "rgba(230,225,212,0.08)";
const AXIS_COLOR = "rgba(230,225,212,0.12)";
const ENTITY_COLOR = "#6dbcdb";      // cyan
const SELECTED_COLOR = "#f0c050";    // amber
const COLLIDER_COLOR = "rgba(155,176,112,0.35)";  // moss
const CAMERA_COLOR = "#6dbcdb";      // cyan
const LABEL_COLOR = "#8a8580";       // ink-3

export default function Viewport({ scene, selectedId, onSelect, activeTool, onTransformCommit, onColliderCommit, engineReady, isPlaying, resolution = "1280 × 720", tilemapEdit, onTilemapEditChange, onSceneChange, projectPath }: Props) {
  const [tab, setTab] = useState<"scene" | "game">("scene");
  const [gizmos, setGizmos] = useState(false);

  // Auto-switch to game tab when play starts, back to scene when stopped
  useEffect(() => {
    if (isPlaying) setTab("game");
  }, [isPlaying]);

  const toggleGizmos = useCallback(async () => {
    const next = !gizmos;
    setGizmos(next);
    try { await invoke("set_gizmos", { enabled: next }); } catch {}
  }, [gizmos]);

  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", minHeight: 0 }}>
      {/* Tab bar */}
      <div style={{
        height: "42px", background: "var(--paper)",
        borderBottom: "1px solid var(--rule)",
        display: "flex", alignItems: "center",
        padding: "0 16px", gap: "20px", flexShrink: 0,
        position: "relative", zIndex: 2,
      }}>
        {(["scene", "game"] as const).map(t => (
          <button key={t} onClick={() => setTab(t)} style={{
            paddingTop: "10px", paddingBottom: "10px",
            fontSize: "12.5px", fontFamily: "var(--font-ui)",
            color: tab === t ? "var(--ink)" : "var(--ink-3)",
            fontWeight: tab === t ? 500 : 400,
            borderBottom: tab === t ? "2px solid var(--ink)" : "2px solid transparent",
            borderTop: "none", borderLeft: "none", borderRight: "none",
            marginBottom: "-1px",
            background: "none",
            cursor: "pointer", textTransform: "capitalize",
          }}>{t}</button>
        ))}
        <div style={{ flex: 1 }} />
        {isPlaying && (
          <span style={{ fontSize: "11px", color: "var(--moss)", fontFamily: "var(--font-mono)", display: "flex", alignItems: "center", gap: "6px" }}>
            <span style={{ width: "6px", height: "6px", background: "var(--moss)", display: "inline-block" }} />
            playing
          </span>
        )}
        <button
          onClick={toggleGizmos}
          title="Toggle gizmos"
          style={{
            height: "22px", padding: "0 8px",
            fontFamily: "var(--font-mono)", fontSize: "10px",
            background: gizmos ? "var(--moss)" : "transparent",
            color: gizmos ? "var(--paper)" : "var(--ink-3)",
            border: `1px solid ${gizmos ? "var(--moss)" : "var(--rule)"}`,
            cursor: "pointer",
            display: "block",
          }}
        >
          gizmos
        </button>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>{resolution}</span>
      </div>

      <div style={{ flex: 1, position: "relative", overflow: "hidden" }}>
        {tab === "scene" ? (
          <SceneView
            scene={scene}
            selectedId={selectedId}
            onSelect={onSelect}
            activeTool={activeTool}
            onTransformCommit={onTransformCommit}
            onColliderCommit={onColliderCommit}
            gizmos={gizmos}
            tilemapEdit={tilemapEdit}
            onTilemapEditChange={onTilemapEditChange}
            onSceneChange={onSceneChange}
            projectPath={projectPath}
          />
        ) : (
          <GameView engineReady={engineReady} isPlaying={isPlaying} />
        )}
      </div>
    </div>
  );
}

// ─── Scene canvas view ──────────────────────────────────────────────────────

interface TransformDraft {
  x: number;
  y: number;
  scale_x: number;
  scale_y: number;
  rotation: number;
}

export interface TransformChange {
  entityId: number;
  before: TransformDraft;
  after: TransformDraft;
}

interface DragState {
  entityId: number;
  tool: Exclude<ActiveTool, "select" | "collider">;
  startMouse: { x: number; y: number };
  startTransform: TransformDraft;
  startAngle: number;
}

type ColliderHandle = "left" | "right" | "top" | "bottom";

interface ColliderDragState {
  entityId: number;
  componentIdx: number;
  handle: ColliderHandle;
  startMouse: { x: number; y: number };
  entityRotation: number;
  startCollider: { width: number; height: number; offset_x: number; offset_y: number };
}

interface ColliderDraft {
  width: number;
  height: number;
  offset_x: number;
  offset_y: number;
}

function encodeTile(paletteId: number, tileIdx: number): number {
  return (((paletteId & 0xffff) << 16) | (tileIdx & 0xffff)) >>> 0;
}

function SceneView({
  scene,
  selectedId,
  onSelect,
  activeTool,
  onTransformCommit,
  onColliderCommit,
  gizmos,
  tilemapEdit,
  onSceneChange,
  projectPath,
}: {
  scene: Scene | null;
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  activeTool: ActiveTool;
  onTransformCommit: (change: TransformChange) => void;
  onColliderCommit: (change: ColliderChange) => void;
  gizmos?: boolean;
  tilemapEdit?: TilemapEdit | null;
  onTilemapEditChange?: (e: TilemapEdit) => void;
  onSceneChange?: () => void;
  projectPath?: string | null;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const cameraRef = useRef<Camera>({ x: 0, y: 0, zoom: 1 });
  const isPanning = useRef(false);
  const lastMouse = useRef({ x: 0, y: 0 });
  const sceneRef = useRef(scene);
  const selectedRef = useRef(selectedId);
  const rafRef = useRef<number>(0);
  const activeToolRef = useRef(activeTool);
  const gizmosRef = useRef(gizmos);
  const dragRef = useRef<DragState | null>(null);
  const draftRef = useRef<Map<number, TransformDraft>>(new Map());
  const colliderDragRef = useRef<ColliderDragState | null>(null);
  const colliderDraftRef = useRef<Map<number, ColliderDraft>>(new Map());
  // Animated sprite: image cache and per-entity animation state
  const imgCacheRef = useRef<Map<string, HTMLImageElement | null>>(new Map());
  const animStateRef = useRef<Map<number, { frame: number; timer: number }>>(new Map());
  const lastDrawTimeRef = useRef(performance.now());
  const debugPathsRef = useRef<Record<string, [number, number][]>>({});

  // Poll debug paths from server when gizmos are on
  useEffect(() => {
    if (!gizmos) { debugPathsRef.current = {}; return; }
    let cancelled = false;
    const poll = async () => {
      if (cancelled) return;
      try {
        const res = await fetch("http://127.0.0.1:7878/debug/paths");
        if (res.ok) debugPathsRef.current = await res.json();
      } catch {}
      if (!cancelled) setTimeout(poll, 100);
    };
    poll();
    return () => { cancelled = true; };
  }, [gizmos]);

  // Tilemap painting state
  const tilemapEditRef = useRef(tilemapEdit);
  const isPaintingTilesRef = useRef(false);
  const tilemapHoverRef = useRef<{ col: number; row: number } | null>(null);
  const tileLayersDraftRef = useRef<{ entityId: number; compIdx: number; layers: unknown[] } | null>(null);
  const tileSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Ghost image for stamp mode
  const stampGhostRef = useRef<{ img: HTMLImageElement | null; w: number; h: number; path: string | null }>({ img: null, w: 0, h: 0, path: null });

  useEffect(() => { sceneRef.current = scene; }, [scene]);
  useEffect(() => { selectedRef.current = selectedId; }, [selectedId]);
  useEffect(() => { activeToolRef.current = activeTool; }, [activeTool]);
  useEffect(() => { gizmosRef.current = gizmos; }, [gizmos]);
  useEffect(() => { tilemapEditRef.current = tilemapEdit; }, [tilemapEdit]);

  const worldToScreen = (wx: number, wy: number, cw: number, ch: number) => ({
    sx: (wx - cameraRef.current.x) * cameraRef.current.zoom + cw / 2,
    sy: (wy - cameraRef.current.y) * cameraRef.current.zoom + ch / 2,
  });

  const screenToWorld = (sx: number, sy: number, cw: number, ch: number) => ({
    wx: (sx - cw / 2) / cameraRef.current.zoom + cameraRef.current.x,
    wy: (sy - ch / 2) / cameraRef.current.zoom + cameraRef.current.y,
  });

  // Load ghost image for stamp mode
  useEffect(() => {
    const te = tilemapEdit;
    if (!te || te.mode !== "stamp" || !te.selectedPrefabPath || !projectPath) {
      stampGhostRef.current = { img: null, w: 0, h: 0, path: null };
      return;
    }
    if (stampGhostRef.current.path === te.selectedPrefabPath) return;
    stampGhostRef.current = { img: null, w: 0, h: 0, path: te.selectedPrefabPath };
    invoke<string>("read_project_file", { projectPath, relativePath: te.selectedPrefabPath })
      .then(text => {
        const node = JSON.parse(text) as { components?: { type: string; texture_path?: string; width?: number; height?: number }[] };
        const sprite = node.components?.find(c => c.type === "Sprite" || c.type === "AnimatedSprite");
        if (!sprite?.texture_path) return;
        const w = sprite.width ?? 16;
        const h = sprite.height ?? 16;
        const img = new Image();
        img.onload = () => { stampGhostRef.current = { img, w, h, path: te.selectedPrefabPath }; };
        img.onerror = () => {};
        img.src = `http://localhost:7878/assets/${sprite.texture_path}`;
      })
      .catch(() => {});
  }, [tilemapEdit?.mode, tilemapEdit?.selectedPrefabPath, projectPath]); // eslint-disable-line react-hooks/exhaustive-deps

  const flushTilePatch = useCallback(async (entityId: number, compIdx: number, layers: unknown[]) => {
    try {
      await invoke("patch_component", { entityId, componentIdx: compIdx, data: { layers } });
      onSceneChange?.();
    } catch {}
  }, [onSceneChange]);

  const paintTile = useCallback((entityId: number, compIdx: number, col: number, row: number) => {
    const te = tilemapEditRef.current;
    const sc = sceneRef.current;
    if (!te || !sc || te.mode === "collision" || te.mode === "stamp") return;
    const entity = sc.entities[String(entityId)];
    if (!entity) return;
    const tmComp = entity.components[compIdx] as { type: "Tilemap"; palettes: unknown[]; layers: { name: string; tiles: number[]; visible: boolean; opacity: number; z_index: number }[]; map_cols: number; map_rows: number } | undefined;
    if (!tmComp || tmComp.type !== "Tilemap") return;

    const draft = tileLayersDraftRef.current;
    const currentLayers = (draft?.entityId === entityId ? draft.layers : tmComp.layers) as typeof tmComp.layers;
    const layerIdx = Math.min(te.layerIdx, currentLayers.length - 1);
    const layer = currentLayers[layerIdx];
    if (!layer) return;

    const idx = row * tmComp.map_cols + col;
    if (idx < 0 || idx >= layer.tiles.length) return;

    const palettes = tmComp.palettes as { name: string; texture_path: string; tileset_cols: number; tileset_rows: number; solid_tiles: number[] }[];
    const pal = palettes[te.paletteIdx];
    const val = te.mode === "erase" ? 0 : (pal ? encodeTile(te.paletteIdx + 1, te.tileIdx) : 0);
    if (te.mode !== "erase" && !pal) return;
    if (layer.tiles[idx] === val) return;

    const newLayers = currentLayers.map((l, i) => {
      if (i !== layerIdx) return l;
      const newTiles = [...l.tiles];
      newTiles[idx] = val;
      return { ...l, tiles: newTiles };
    });

    tileLayersDraftRef.current = { entityId, compIdx, layers: newLayers };
    if (tileSaveTimerRef.current) clearTimeout(tileSaveTimerRef.current);
    tileSaveTimerRef.current = setTimeout(() => {
      const d = tileLayersDraftRef.current;
      if (d) { flushTilePatch(d.entityId, d.compIdx, d.layers); tileLayersDraftRef.current = null; }
    }, 150);
  }, [flushTilePatch]);

  const getTilemapAtPoint = useCallback((wx: number, wy: number): { entity: Entity; compIdx: number; col: number; row: number } | null => {
    const sc = sceneRef.current;
    const selId = selectedRef.current;
    if (!sc || selId === null) return null;
    const entity = sc.entities[String(selId)];
    if (!entity) return null;
    const compIdx = entity.components.findIndex(c => c.type === "Tilemap");
    if (compIdx < 0) return null;
    const tm = entity.components[compIdx] as { type: "Tilemap"; tile_width: number; tile_height: number; map_cols: number; map_rows: number };
    const worldPos = resolveWorldPos(sc, entity);
    const col = Math.floor((wx - worldPos.x) / tm.tile_width);
    const row = Math.floor((wy - worldPos.y) / tm.tile_height);
    if (col < 0 || row < 0 || col >= tm.map_cols || row >= tm.map_rows) return null;
    return { entity, compIdx, col, row };
  }, []);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const cw = canvas.width;
    const ch = canvas.height;
    const cam = cameraRef.current;
    const sc = sceneRef.current;
    const selId = selectedRef.current;
    const now = performance.now();
    const dt = Math.min((now - lastDrawTimeRef.current) / 1000, 0.1);
    lastDrawTimeRef.current = now;

    ctx.fillStyle = BG;
    ctx.fillRect(0, 0, cw, ch);
    ctx.imageSmoothingEnabled = false;

    // — Grid —
    const rawStep = 100;
    let step = rawStep;
    while (step * cam.zoom < 20) step *= 5;
    while (step * cam.zoom > 200) step /= 5;

    const screenStep = step * cam.zoom;
    const worldLeft = cam.x - cw / 2 / cam.zoom;
    const worldRight = cam.x + cw / 2 / cam.zoom;
    const worldTop = cam.y - ch / 2 / cam.zoom;
    const worldBottom = cam.y + ch / 2 / cam.zoom;

    const startX = Math.floor(worldLeft / step) * step;
    const startY = Math.floor(worldTop / step) * step;

    // Minor lines
    ctx.strokeStyle = GRID_MINOR;
    ctx.lineWidth = 1;
    for (let wx = startX; wx <= worldRight + step; wx += step) {
      const sx = (wx - cam.x) * cam.zoom + cw / 2;
      ctx.beginPath(); ctx.moveTo(sx, 0); ctx.lineTo(sx, ch); ctx.stroke();
    }
    for (let wy = startY; wy <= worldBottom + step; wy += step) {
      const sy = (wy - cam.y) * cam.zoom + ch / 2;
      ctx.beginPath(); ctx.moveTo(0, sy); ctx.lineTo(cw, sy); ctx.stroke();
    }

    // Major lines (every 5 minor)
    ctx.strokeStyle = GRID_MAJOR;
    const majorStep = step * 5;
    for (let wx = Math.floor(worldLeft / majorStep) * majorStep; wx <= worldRight + majorStep; wx += majorStep) {
      const sx = (wx - cam.x) * cam.zoom + cw / 2;
      ctx.beginPath(); ctx.moveTo(sx, 0); ctx.lineTo(sx, ch); ctx.stroke();
    }
    for (let wy = Math.floor(worldTop / majorStep) * majorStep; wy <= worldBottom + majorStep; wy += majorStep) {
      const sy = (wy - cam.y) * cam.zoom + ch / 2;
      ctx.beginPath(); ctx.moveTo(0, sy); ctx.lineTo(cw, sy); ctx.stroke();
    }

    // — World axes —
    const ox = (0 - cam.x) * cam.zoom + cw / 2;
    const oy = (0 - cam.y) * cam.zoom + ch / 2;
    ctx.strokeStyle = AXIS_COLOR;
    ctx.lineWidth = 1;
    if (ox >= 0 && ox <= cw) { ctx.beginPath(); ctx.moveTo(ox, 0); ctx.lineTo(ox, ch); ctx.stroke(); }
    if (oy >= 0 && oy <= ch) { ctx.beginPath(); ctx.moveTo(0, oy); ctx.lineTo(cw, oy); ctx.stroke(); }

    // — Coordinate labels on major lines —
    if (screenStep > 60) {
      ctx.font = "9px monospace";
      ctx.fillStyle = "rgba(255,255,255,0.18)";
      for (let wx = Math.floor(worldLeft / majorStep) * majorStep; wx <= worldRight + majorStep; wx += majorStep) {
        if (Math.abs(wx) < 0.01) continue;
        const sx = (wx - cam.x) * cam.zoom + cw / 2;
        if (sx >= 0 && sx <= cw) ctx.fillText(String(Math.round(wx)), sx + 2, oy > 10 && oy < ch - 2 ? oy - 2 : ch - 2);
      }
      for (let wy = Math.floor(worldTop / majorStep) * majorStep; wy <= worldBottom + majorStep; wy += majorStep) {
        if (Math.abs(wy) < 0.01) continue;
        const sy = (wy - cam.y) * cam.zoom + ch / 2;
        if (sy >= 0 && sy <= ch) ctx.fillText(String(Math.round(wy)), ox > 10 && ox < cw - 20 ? ox + 3 : 3, sy - 2);
      }
    }

    // — Entities — sorted by z_index for correct depth order
    if (sc) {
      type DrawItem = { z: number; entityId: number; layerIdx: number | null };
      const drawList: DrawItem[] = [];
      for (const entity of Object.values(sc.entities)) {
        const transform = entity.components.find(c => c.type === "Transform") as { type: "Transform"; z_index?: number } | undefined;
        const baseZ = transform?.z_index ?? 0;
        const tm = entity.components.find(c => c.type === "Tilemap") as { type: "Tilemap"; layers: { visible: boolean; z_index?: number }[] } | undefined;
        if (tm) {
          tm.layers.forEach((layer, idx) => {
            if (layer.visible) drawList.push({ z: baseZ + (layer.z_index ?? 0), entityId: entity.id, layerIdx: idx });
          });
        } else {
          drawList.push({ z: baseZ, entityId: entity.id, layerIdx: null });
        }
      }
      drawList.sort((a, b) => a.z !== b.z ? a.z - b.z : a.entityId - b.entityId);
      for (const item of drawList) {
        const entity = sc.entities[String(item.entityId)];
        if (!entity) continue;
        drawEntity(ctx, entity, sc, selId, cw, ch, cam, worldToScreen, draftRef.current.get(entity.id), activeToolRef.current, colliderDraftRef.current.get(entity.id), imgCacheRef.current, animStateRef.current, dt, gizmosRef.current, item.layerIdx);
      }
    }

    // — Tilemap hover overlay —
    const te = tilemapEditRef.current;
    const hover = tilemapHoverRef.current;
    if (te && hover && sc && selId !== null) {
      const hovEntity = sc.entities[String(selId)];
      const hovCompIdx = hovEntity ? hovEntity.components.findIndex(c => c.type === "Tilemap") : -1;
      const hovTm = hovCompIdx >= 0 ? hovEntity!.components[hovCompIdx] as { type: "Tilemap"; palettes: { name: string; texture_path: string; tileset_cols: number; tileset_rows: number; margin: number; spacing: number; solid_tiles: number[] }[]; tile_width: number; tile_height: number; map_cols: number; map_rows: number } : null;
      const hovTransform = hovEntity ? getTransform(hovEntity) : null;
      if (hovTm && hovTransform) {
        const hovWorldPos = hovEntity ? resolveWorldPos(sc, hovEntity) : hovTransform;
        const { sx: tmSx, sy: tmSy } = worldToScreen(hovWorldPos.x, hovWorldPos.y, cw, ch);
        const tilePxW = hovTm.tile_width * cam.zoom;
        const tilePxH = hovTm.tile_height * cam.zoom;
        const hx = tmSx + hover.col * tilePxW;
        const hy = tmSy + hover.row * tilePxH;

        ctx.save();
        if (te.mode === "draw") {
          const hovPal = hovTm.palettes[te.paletteIdx];
          const hovImg = hovPal ? imgCacheRef.current.get(hovPal.texture_path) : null;
          if (hovImg) {
            const cols = Math.max(1, hovPal!.tileset_cols);
            const rows = Math.max(1, hovPal!.tileset_rows);
            const m = hovPal!.margin ?? 0;
            const s = hovPal!.spacing ?? 0;
            const iw = hovImg.naturalWidth; const ih = hovImg.naturalHeight;
            let srcX: number, srcY: number, srcW: number, srcH: number;
            if (m === 0 && s === 0) {
              srcW = iw / cols; srcH = ih / rows;
              srcX = (te.tileIdx % cols) * srcW; srcY = Math.floor(te.tileIdx / cols) * srcH;
            } else {
              srcW = (iw - 2 * m - s * (cols - 1)) / cols; srcH = (ih - 2 * m - s * (rows - 1)) / rows;
              srcX = m + (te.tileIdx % cols) * (srcW + s); srcY = m + Math.floor(te.tileIdx / cols) * (srcH + s);
            }
            ctx.globalAlpha = 0.75;
            ctx.imageSmoothingEnabled = false;
            ctx.drawImage(hovImg, srcX, srcY, srcW, srcH, hx, hy, Math.round(tilePxW), Math.round(tilePxH));
            ctx.globalAlpha = 1;
          }
          ctx.strokeStyle = "rgba(180,140,60,0.9)";
          ctx.lineWidth = 2;
          ctx.strokeRect(hx + 1, hy + 1, tilePxW - 2, tilePxH - 2);
        } else if (te.mode === "erase") {
          ctx.fillStyle = "rgba(220,60,60,0.25)";
          ctx.fillRect(hx, hy, tilePxW, tilePxH);
          ctx.strokeStyle = "rgba(220,60,60,0.9)";
          ctx.lineWidth = 2;
          ctx.strokeRect(hx + 1, hy + 1, tilePxW - 2, tilePxH - 2);
        } else if (te.mode === "stamp") {
          const ghost = stampGhostRef.current;
          if (ghost.img) {
            ctx.globalAlpha = 0.7;
            ctx.imageSmoothingEnabled = false;
            ctx.drawImage(ghost.img, hx, hy, Math.round(tilePxW), Math.round(tilePxH));
            ctx.globalAlpha = 1;
          }
          ctx.strokeStyle = "rgba(180,140,60,0.9)";
          ctx.lineWidth = 2;
          ctx.strokeRect(hx + 1, hy + 1, tilePxW - 2, tilePxH - 2);
        }
        ctx.restore();
      }
    }

    // — Debug paths (gizmo) —
    if (gizmosRef.current) {
      const paths = debugPathsRef.current;
      const colors = ["#ff6b35", "#ffd166", "#06d6a0", "#118ab2", "#ef476f"];
      let colorIdx = 0;
      for (const pts of Object.values(paths)) {
        if (!pts || pts.length < 2) continue;
        const color = colors[colorIdx % colors.length];
        colorIdx++;
        ctx.save();
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.setLineDash([4, 3]);
        ctx.globalAlpha = 0.85;
        ctx.beginPath();
        for (let i = 0; i < pts.length; i++) {
          const s = worldToScreen(pts[i][0], pts[i][1], cw, ch);
          if (i === 0) ctx.moveTo(s.sx, s.sy); else ctx.lineTo(s.sx, s.sy);
        }
        ctx.stroke();
        ctx.setLineDash([]);
        for (let i = 0; i < pts.length; i++) {
          const s = worldToScreen(pts[i][0], pts[i][1], cw, ch);
          ctx.fillStyle = i === 0 ? "white" : color;
          ctx.beginPath();
          ctx.arc(s.sx, s.sy, 3, 0, Math.PI * 2);
          ctx.fill();
        }
        ctx.restore();
      }
    }

    // — Zoom indicator —
    ctx.font = "10px monospace";
    ctx.fillStyle = "rgba(255,255,255,0.2)";
    ctx.fillText(`${cam.zoom.toFixed(2)}x`, cw - 44, ch - 6);
  }, []);

  // Render loop
  useEffect(() => {
    let running = true;
    const loop = () => {
      draw();
      if (running) rafRef.current = requestAnimationFrame(loop);
    };
    rafRef.current = requestAnimationFrame(loop);
    return () => { running = false; cancelAnimationFrame(rafRef.current); };
  }, [draw]);

  // Resize canvas to container
  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;
    const observer = new ResizeObserver(() => {
      canvas.width = container.clientWidth;
      canvas.height = container.clientHeight;
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Mouse handlers
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const cam = cameraRef.current;
    const zoomFactor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
    const newZoom = Math.min(Math.max(cam.zoom * zoomFactor, 0.05), 50);
    // Zoom toward mouse cursor
    const wx = (mx - canvas.width / 2) / cam.zoom + cam.x;
    const wy = (my - canvas.height / 2) / cam.zoom + cam.y;
    cameraRef.current = {
      x: wx - (mx - canvas.width / 2) / newZoom,
      y: wy - (my - canvas.height / 2) / newZoom,
      zoom: newZoom,
    };
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button === 1 || e.button === 2) {
      isPanning.current = true;
      lastMouse.current = { x: e.clientX, y: e.clientY };
      e.preventDefault();
      return;
    }

    if (e.button !== 0) {
      return;
    }

    const canvas = canvasRef.current;
    const sc = sceneRef.current;
    if (!canvas || !sc) {
      return;
    }
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const { wx, wy } = screenToWorld(mx, my, canvas.width, canvas.height);
    const tool = activeToolRef.current;

    // Tilemap painting mode
    const te = tilemapEditRef.current;
    if (te) {
      const hit = getTilemapAtPoint(wx, wy);
      if (hit) {
        if (te.mode === "stamp") {
          // Stamp: store LOCAL coords (offset from tilemap origin) so child follows parent on move.
          const selId = selectedRef.current;
          if (te.selectedPrefabPath && projectPath && selId !== null) {
            const tm = hit.entity.components[hit.compIdx] as { tile_width: number; tile_height: number };
            const localX = (hit.col + 0.5) * tm.tile_width;
            const localY = (hit.row + 0.5) * tm.tile_height;
            invoke<number>("instantiate_prefab", { projectPath, prefabPath: te.selectedPrefabPath, parentId: selId })
              .then(newId => invoke("patch_transform", { entityId: newId, x: localX, y: localY, scaleX: 1, scaleY: 1, rotation: 0 }))
              .then(() => onSceneChange?.())
              .catch(err => console.error("stamp failed:", err));
          }
        } else {
          isPaintingTilesRef.current = true;
          paintTile(hit.entity.id, hit.compIdx, hit.col, hit.row);
        }
        e.preventDefault();
        return;
      }
    }

    // In collider mode: check handles on the selected entity FIRST, before any hit test.
    // This ensures handles always win over selecting a different entity underneath.
    if (tool === "collider") {
      const selectedEntity = selectedRef.current !== null ? sc.entities[String(selectedRef.current)] : null;
      if (selectedEntity) {
        const collider = selectedEntity.components.find(c => c.type === "Collider") as
          | { type: "Collider"; width: number; height: number; offset_x: number; offset_y: number }
          | undefined;
        const transform = getTransform(selectedEntity);
        const componentIdx = selectedEntity.components.findIndex(c => c.type === "Collider");
        if (collider && transform && componentIdx >= 0) {
          const cx = transform.x + collider.offset_x;
          const cy = transform.y + collider.offset_y;
          const hw = collider.width * 0.5;
          const hh = collider.height * 0.5;
          const rot = transform.rotation;
          const cosR = Math.cos(rot);
          const sinR = Math.sin(rot);
          const relX = wx - cx;
          const relY = wy - cy;
          // Rotate click into entity-local space
          const localX = relX * cosR + relY * sinR;
          const localY = -relX * sinR + relY * cosR;
          const handles: [ColliderHandle, number, number][] = [
            ["left",   -hw, 0],
            ["right",  hw,  0],
            ["top",    0,   -hh],
            ["bottom", 0,   hh],
          ];
          const hitThreshold = 10 / cameraRef.current.zoom;
          for (const [handle, hx, hy] of handles) {
            if (Math.abs(localX - hx) <= hitThreshold && Math.abs(localY - hy) <= hitThreshold) {
              colliderDragRef.current = {
                entityId: selectedRef.current!,
                componentIdx,
                handle,
                startMouse: { x: wx, y: wy },
                entityRotation: rot,
                startCollider: {
                  width: collider.width,
                  height: collider.height,
                  offset_x: collider.offset_x,
                  offset_y: collider.offset_y,
                },
              };
              colliderDraftRef.current.set(selectedRef.current!, {
                width: collider.width,
                height: collider.height,
                offset_x: collider.offset_x,
                offset_y: collider.offset_y,
              });
              e.preventDefault();
              return;
            }
          }
        }
      }
      // Clicked outside any handle — allow selecting a different entity
      const hit = hitTest(sc, wx, wy);
      if (hit !== null) onSelect(hit);
      return;
    }

    const hit = hitTest(sc, wx, wy);
    if (hit !== null && hit !== selectedRef.current) {
      onSelect(hit);
    }

    const targetId = hit ?? selectedRef.current;
    if (tool === "select" || targetId === null) {
      return;
    }

    const entity = sc.entities[String(targetId)];
    if (!entity) return;

    const transform = getTransform(entity);
    if (!transform) return;

    dragRef.current = {
      entityId: targetId,
      tool,
      startMouse: { x: wx, y: wy },
      startTransform: { ...transform },
      startAngle: Math.atan2(wy - transform.y, wx - transform.x),
    };
    draftRef.current.set(targetId, { ...transform });
    e.preventDefault();
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    // Tilemap hover tracking
    if (tilemapEditRef.current) {
      const canvas = canvasRef.current;
      if (canvas) {
        const rect = canvas.getBoundingClientRect();
        const mx = e.clientX - rect.left;
        const my = e.clientY - rect.top;
        const { wx, wy } = screenToWorld(mx, my, canvas.width, canvas.height);
        const hit = getTilemapAtPoint(wx, wy);
        tilemapHoverRef.current = hit ? { col: hit.col, row: hit.row } : null;
        if (isPaintingTilesRef.current && hit && tilemapEditRef.current!.mode !== "stamp") {
          paintTile(hit.entity.id, hit.compIdx, hit.col, hit.row);
        }
      }
    } else {
      tilemapHoverRef.current = null;
    }

    const colliderDrag = colliderDragRef.current;
    if (colliderDrag) {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      const { wx, wy } = screenToWorld(mx, my, canvas.width, canvas.height);
      const dx = wx - colliderDrag.startMouse.x;
      const dy = wy - colliderDrag.startMouse.y;
      // Project world delta onto entity local axes
      const rot = colliderDrag.entityRotation;
      const cosR = Math.cos(rot);
      const sinR = Math.sin(rot);
      const ldx = dx * cosR + dy * sinR;   // component along local X
      const ldy = -dx * sinR + dy * cosR;  // component along local Y
      const s = colliderDrag.startCollider;
      let { width, height, offset_x, offset_y } = s;
      // For each handle: grow the dimension, then move center by half in world space.
      // Local X axis in world space: (cosR, sinR)
      // Local Y axis in world space: (-sinR, cosR)
      switch (colliderDrag.handle) {
        case "right": {
          width = Math.max(1, s.width + ldx);
          const h = ldx * 0.5;
          offset_x = s.offset_x + h * cosR; offset_y = s.offset_y + h * sinR;
          break;
        }
        case "left": {
          width = Math.max(1, s.width - ldx);
          const h = ldx * 0.5;
          offset_x = s.offset_x + h * cosR; offset_y = s.offset_y + h * sinR;
          break;
        }
        case "bottom": {
          height = Math.max(1, s.height + ldy);
          const h = ldy * 0.5;
          offset_x = s.offset_x - h * sinR; offset_y = s.offset_y + h * cosR;
          break;
        }
        case "top": {
          height = Math.max(1, s.height - ldy);
          const h = ldy * 0.5;
          offset_x = s.offset_x - h * sinR; offset_y = s.offset_y + h * cosR;
          break;
        }
      }
      colliderDraftRef.current.set(colliderDrag.entityId, { width, height, offset_x, offset_y });
      return;
    }

    const drag = dragRef.current;
    if (drag) {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      const { wx, wy } = screenToWorld(mx, my, canvas.width, canvas.height);
      const next = nextTransformForDrag(drag, wx, wy, e.shiftKey);
      draftRef.current.set(drag.entityId, next);
      return;
    }

    if (isPanning.current) {
      const dx = e.clientX - lastMouse.current.x;
      const dy = e.clientY - lastMouse.current.y;
      lastMouse.current = { x: e.clientX, y: e.clientY };
      const cam = cameraRef.current;
      cameraRef.current = { ...cam, x: cam.x - dx / cam.zoom, y: cam.y - dy / cam.zoom };
    }
  };

  const handleMouseUp = (e: React.MouseEvent) => {
    if (isPaintingTilesRef.current) {
      isPaintingTilesRef.current = false;
      // Flush any pending tile save immediately
      if (tileSaveTimerRef.current) {
        clearTimeout(tileSaveTimerRef.current);
        tileSaveTimerRef.current = null;
      }
      const d = tileLayersDraftRef.current;
      if (d) { flushTilePatch(d.entityId, d.compIdx, d.layers); tileLayersDraftRef.current = null; }
      return;
    }

    const colliderDrag = colliderDragRef.current;
    if (colliderDrag) {
      colliderDragRef.current = null;
      const next = colliderDraftRef.current.get(colliderDrag.entityId);
      colliderDraftRef.current.delete(colliderDrag.entityId);
      if (next) {
        onColliderCommit({
          entityId: colliderDrag.entityId,
          componentIdx: colliderDrag.componentIdx,
          before: colliderDrag.startCollider,
          after: next,
        });
      }
      return;
    }

    const drag = dragRef.current;
    if (drag) {
      dragRef.current = null;
      const next = draftRef.current.get(drag.entityId);
      draftRef.current.delete(drag.entityId);
      if (next) {
        onTransformCommit({
          entityId: drag.entityId,
          before: drag.startTransform,
          after: next,
        });
      }
      return;
    }

    if (isPanning.current) { isPanning.current = false; return; }
    if (e.button !== 0) return;

    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const { wx, wy } = screenToWorld(mx, my, canvas.width, canvas.height);

    if (!scene) { onSelect(null); return; }

    onSelect(hitTest(scene, wx, wy));
  };

  const tilemapCursor = tilemapEdit
    ? (tilemapEdit.mode === "stamp" ? (tilemapEdit.selectedPrefabPath ? "cell" : "not-allowed") : "crosshair")
    : null;

  return (
    <div ref={containerRef} style={{ width: "100%", height: "100%", cursor: tilemapCursor ?? (activeTool === "select" ? "crosshair" : "grab") }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={() => { tilemapHoverRef.current = null; }}
      onContextMenu={e => e.preventDefault()}
      onWheel={handleWheel}
    >
      <canvas ref={canvasRef} style={{ display: "block" }} />
    </div>
  );
}

function getTransform(entity: Entity): TransformDraft | null {
  const transform = entity.components.find(c => c.type === "Transform") as
    | { type: "Transform"; x: number; y: number; scale_x: number; scale_y: number; rotation: number }
    | undefined;
  return transform ? {
    x: transform.x,
    y: transform.y,
    scale_x: transform.scale_x,
    scale_y: transform.scale_y,
    rotation: transform.rotation,
  } : null;
}

// Walk the parent chain to compute world-space position.
// Each entity's (x, y) is a local offset from its parent's world position.
function resolveWorldPos(scene: Scene, entity: Entity, depth = 0): { x: number; y: number } {
  if (depth > 16) return { x: 0, y: 0 };
  const t = getTransform(entity);
  if (!t) return { x: 0, y: 0 };
  if (entity.parent === null || entity.parent === undefined) return { x: t.x, y: t.y };
  const parentEntity = scene.entities[String(entity.parent)];
  if (!parentEntity) return { x: t.x, y: t.y };
  const parentWorld = resolveWorldPos(scene, parentEntity, depth + 1);
  return { x: parentWorld.x + t.x, y: parentWorld.y + t.y };
}

function hitTest(scene: Scene, wx: number, wy: number): number | null {
  let hit: number | null = null;
  for (const entity of Object.values(scene.entities)) {
    const transform = getTransform(entity);
    const sprite = entity.components.find(c => c.type === "Sprite") as { type: "Sprite"; width: number; height: number } | undefined;
    const anim = entity.components.find(c => c.type === "AnimatedSprite") as { type: "AnimatedSprite"; width: number; height: number } | undefined;
    const tm = entity.components.find(c => c.type === "Tilemap") as { type: "Tilemap"; map_cols: number; map_rows: number; tile_width: number; tile_height: number } | undefined;
    const camera = entity.components.find(c => c.type === "Camera") as { type: "Camera"; zoom: number } | undefined;
    if (!transform) continue;
    const world = resolveWorldPos(scene, entity);
    if (tm) {
      const tmW = tm.map_cols * tm.tile_width;
      const tmH = tm.map_rows * tm.tile_height;
      if (wx >= world.x && wx <= world.x + tmW && wy >= world.y && wy <= world.y + tmH) {
        hit = entity.id;
      }
      continue;
    }
    const visW = sprite?.width ?? anim?.width;
    const visH = sprite?.height ?? anim?.height;
    const hw = visW ? Math.abs(visW * transform.scale_x) * 0.5 : camera ? 12 : Math.max(Math.abs(transform.scale_x) * 16, 12);
    const hh = visH ? Math.abs(visH * transform.scale_y) * 0.5 : camera ? 12 : Math.max(Math.abs(transform.scale_y) * 16, 12);
    if (wx >= world.x - hw && wx <= world.x + hw && wy >= world.y - hh && wy <= world.y + hh) {
      hit = entity.id;
    }
  }
  return hit;
}

function snap(value: number, step: number): number {
  return Math.round(value / step) * step;
}

function nextTransformForDrag(drag: DragState, wx: number, wy: number, snapping: boolean): TransformDraft {
  const dx = wx - drag.startMouse.x;
  const dy = wy - drag.startMouse.y;
  const next = { ...drag.startTransform };

  if (drag.tool === "move") {
    next.x = drag.startTransform.x + dx;
    next.y = drag.startTransform.y + dy;
    if (snapping) {
      next.x = snap(next.x, 16);
      next.y = snap(next.y, 16);
    }
  } else if (drag.tool === "scale") {
    const factor = Math.max(0.05, 1 + (dx + dy) * 0.01);
    next.scale_x = drag.startTransform.scale_x * factor;
    next.scale_y = drag.startTransform.scale_y * factor;
    if (snapping) {
      next.scale_x = snap(next.scale_x, 0.1);
      next.scale_y = snap(next.scale_y, 0.1);
    }
  } else if (drag.tool === "rotate") {
    const angle = Math.atan2(wy - drag.startTransform.y, wx - drag.startTransform.x);
    next.rotation = drag.startTransform.rotation + angle - drag.startAngle;
    if (snapping) {
      next.rotation = snap(next.rotation, Math.PI / 12);
    }
  }

  return next;
}

function drawEntity(
  ctx: CanvasRenderingContext2D,
  entity: Entity,
  scene: Scene,
  selectedId: number | null,
  cw: number,
  ch: number,
  cam: Camera,
  worldToScreen: (wx: number, wy: number, cw: number, ch: number) => { sx: number; sy: number },
  draft?: TransformDraft,
  activeTool?: ActiveTool,
  colliderDraft?: ColliderDraft,
  imgCache?: Map<string, HTMLImageElement | null>,
  animState?: Map<number, { frame: number; timer: number }>,
  dt?: number,
  gizmos?: boolean,
  onlyLayerIdx?: number | null,
) {
  const transform = entity.components.find(c => c.type === "Transform") as
    | { type: "Transform"; x: number; y: number; scale_x: number; scale_y: number; rotation: number }
    | undefined;
  const sprite = entity.components.find(c => c.type === "Sprite") as
    | { type: "Sprite"; width: number; height: number; color: [number, number, number, number] }
    | undefined;
  const animSprite = entity.components.find(c => c.type === "AnimatedSprite") as
    | { type: "AnimatedSprite"; texture_path: string; cols: number; rows: number; width: number; height: number; tint: [number,number,number,number]; clips: { name: string; start_frame: number; end_frame: number; fps: number; looping: boolean }[]; default_clip: string; flip_x: boolean; flip_y: boolean }
    | undefined;
  const tilemap = entity.components.find(c => c.type === "Tilemap") as
    | { type: "Tilemap"; palettes: { name: string; texture_path: string; tileset_cols: number; tileset_rows: number; margin: number; spacing: number; solid_tiles: number[] }[]; layers: { name: string; tiles: number[]; visible: boolean; opacity: number; z_index?: number }[]; tile_width: number; tile_height: number; map_cols: number; map_rows: number; tint: [number,number,number,number] }
    | undefined;
  const collider = entity.components.find(c => c.type === "Collider") as
    | { type: "Collider"; width: number; height: number; offset_x: number; offset_y: number }
    | undefined;
  const cameraComp = entity.components.find(c => c.type === "Camera") as
    | { type: "Camera"; active?: boolean; zoom: number }
    | undefined;

  const isSelected = entity.id === selectedId;
  // Use draft for the dragged entity; otherwise resolve world position via parent chain.
  const activeTransform = draft ?? transform;
  const worldPos = draft ? { x: draft.x, y: draft.y } : resolveWorldPos(scene, entity);
  const tx = worldPos.x;
  const ty = worldPos.y;
  const { sx, sy } = worldToScreen(tx, ty, cw, ch);

  if (!activeTransform) {
    // No transform — show a small indicator at origin
    const { sx: ox, sy: oy } = worldToScreen(0, 0, cw, ch);
    ctx.strokeStyle = isSelected ? SELECTED_COLOR : "rgba(160,160,160,0.5)";
    ctx.lineWidth = isSelected ? 2 : 1;
    ctx.beginPath();
    ctx.arc(ox, oy, 5, 0, Math.PI * 2);
    ctx.stroke();
    return;
  }

  if (cameraComp) {
    drawCameraFrame(ctx, entity.name, sx, sy, activeTransform.rotation, cameraComp.zoom, cam.zoom, cameraComp.active ?? true, isSelected);
    if (!sprite && !animSprite && !tilemap && !collider) return;
  }

  if (tilemap) {
    const tmW = tilemap.map_cols * tilemap.tile_width;
    const tmH = tilemap.map_rows * tilemap.tile_height;
    const screenTmW = tmW * cam.zoom;
    const screenTmH = tmH * cam.zoom;
    const tilePxW = tilemap.tile_width * cam.zoom;
    const tilePxH = tilemap.tile_height * cam.zoom;

    // Ensure all palette textures are loaded into imgCache
    if (imgCache) {
      for (const pal of tilemap.palettes) {
        if (!pal.texture_path) continue;
        const cacheKey = pal.texture_path;
        if (imgCache.get(cacheKey) === undefined) {
          const el = new Image();
          el.onload = () => imgCache.set(cacheKey, el);
          el.onerror = () => imgCache.set(cacheKey, null);
          imgCache.set(cacheKey, null);
          el.src = pal.texture_path.startsWith("/") ? pal.texture_path : `http://localhost:7878/assets/${pal.texture_path}`;
        }
      }
    }

    ctx.save();
    ctx.translate(sx, sy);
    ctx.rotate(activeTransform.rotation);

    const tint = tilemap.tint;
    const tileDrawW = Math.round(tilePxW);
    const tileDrawH = Math.round(tilePxH);

    // Render layer(s) — when onlyLayerIdx is set, draw just that one layer
    const layersToRender = (tilemap.layers ?? []).map((layer, idx) => ({ layer, idx }))
      .filter(({ layer, idx }) => layer.visible && (onlyLayerIdx == null || idx === onlyLayerIdx));
    for (const { layer } of layersToRender) {
      if (!layer.visible) continue;
      ctx.globalAlpha = (layer.opacity ?? 1) * tint[3];
      for (let r = 0; r < tilemap.map_rows; r++) {
        for (let c = 0; c < tilemap.map_cols; c++) {
          const cell = layer.tiles[r * tilemap.map_cols + c] ?? 0;
          if (cell === 0) continue;
          const paletteId = (cell >>> 16) & 0xffff;
          const tileIdx = cell & 0xffff;
          if (paletteId === 0) continue;
          const pal = tilemap.palettes[paletteId - 1];
          if (!pal) continue;
          const img = imgCache?.get(pal.texture_path);
          if (!img) continue;
          const cols = Math.max(1, pal.tileset_cols);
          const rows = Math.max(1, pal.tileset_rows);
          const m = pal.margin ?? 0;
          const s = pal.spacing ?? 0;
          const iw = img.naturalWidth; const ih = img.naturalHeight;
          let srcX: number, srcY: number, srcW: number, srcH: number;
          if (m === 0 && s === 0) {
            srcW = iw / cols; srcH = ih / rows;
            srcX = (tileIdx % cols) * srcW; srcY = Math.floor(tileIdx / cols) * srcH;
          } else {
            srcW = (iw - 2 * m - s * (cols - 1)) / cols; srcH = (ih - 2 * m - s * (rows - 1)) / rows;
            srcX = m + (tileIdx % cols) * (srcW + s); srcY = m + Math.floor(tileIdx / cols) * (srcH + s);
          }
          ctx.drawImage(img, srcX, srcY, srcW, srcH, Math.round(c * tilePxW), Math.round(r * tilePxH), tileDrawW, tileDrawH);
        }
      }
    }
    ctx.globalAlpha = 1;

    // Overlays only draw on the last (or only) layer call to avoid duplication
    const visibleLayers = (tilemap.layers ?? []).filter(l => l.visible);
    const isLastLayerCall = onlyLayerIdx == null
      || onlyLayerIdx === (tilemap.layers ?? []).reduce((last, l, i) => l.visible ? i : last, -1);

    if (isLastLayerCall) {
      if (visibleLayers.length === 0) {
        ctx.fillStyle = "rgba(77,120,180,0.20)";
        ctx.fillRect(0, 0, screenTmW, screenTmH);
      }

      // Solid tile overlay (gizmos) — check all layers
      if (gizmos) {
        ctx.fillStyle = "rgba(220,60,60,0.35)";
        for (const layer of (tilemap.layers ?? [])) {
          for (let r = 0; r < tilemap.map_rows; r++) {
            for (let c = 0; c < tilemap.map_cols; c++) {
              const cell = layer.tiles[r * tilemap.map_cols + c] ?? 0;
              if (cell === 0) continue;
              const paletteId = (cell >>> 16) & 0xffff;
              const tileIdx = cell & 0xffff;
              if (paletteId === 0) continue;
              const pal = tilemap.palettes[paletteId - 1];
              if (pal?.solid_tiles.includes(tileIdx)) ctx.fillRect(c * tilePxW, r * tilePxH, tilePxW, tilePxH);
            }
          }
        }
      }

      ctx.strokeStyle = isSelected ? SELECTED_COLOR : "rgba(77,120,180,0.5)";
      ctx.lineWidth = isSelected ? 2 : 1;
      ctx.strokeRect(0, 0, screenTmW, screenTmH);
    }

    ctx.restore();

    if (isSelected && activeTool && activeTool !== "select") {
      drawToolGizmo(ctx, sx, sy, activeTool, cam.zoom);
    }
    return;
  }


  const visW = sprite?.width ?? animSprite?.width;
  const visH = sprite?.height ?? animSprite?.height;
  const hw = visW ? visW * activeTransform.scale_x * 0.5 : Math.max(activeTransform.scale_x * 16, 1);
  const hh = visH ? visH * activeTransform.scale_y * 0.5 : Math.max(activeTransform.scale_y * 16, 1);

  const screenW = hw * 2 * cam.zoom;
  const screenH = hh * 2 * cam.zoom;

  ctx.save();
  ctx.translate(sx, sy);
  ctx.rotate(activeTransform.rotation);

  if (animSprite && imgCache && animState) {
    // Advance animation timer
    const clip = animSprite.clips.find(c => c.name === animSprite.default_clip) ?? animSprite.clips[0];
    if (clip) {
      const frameDur = clip.fps > 0 ? 1 / clip.fps : 0.1;
      const frameCount = clip.end_frame - clip.start_frame + 1;
      let state = animState.get(entity.id);
      if (!state) { state = { frame: 0, timer: 0 }; animState.set(entity.id, state); }
      if (dt) {
        state.timer += dt;
        while (state.timer >= frameDur) {
          state.timer -= frameDur;
          state.frame = (state.frame + 1) % frameCount;
        }
      }
      const absFrame = clip.start_frame + state.frame;
      const col = absFrame % animSprite.cols;
      const row = Math.floor(absFrame / animSprite.cols);
      const uvW = 1 / animSprite.cols;
      const uvH = 1 / animSprite.rows;

      // Load image if not cached
      let img = imgCache.get(animSprite.texture_path);
      if (img === undefined && animSprite.texture_path) {
        const el = new Image();
        el.onload = () => imgCache.set(animSprite.texture_path, el);
        el.onerror = () => imgCache.set(animSprite.texture_path, null);
        imgCache.set(animSprite.texture_path, null); // mark as loading
        el.src = animSprite.texture_path.startsWith("/") ? animSprite.texture_path : `http://localhost:7878/assets/${animSprite.texture_path}`;
        img = null;
      }

      if (img) {
        const sx2 = col * uvW * img.naturalWidth;
        const sy2 = row * uvH * img.naturalHeight;
        const sw = uvW * img.naturalWidth;
        const sh = uvH * img.naturalHeight;
        if (animSprite.flip_x || animSprite.flip_y) {
          ctx.scale(animSprite.flip_x ? -1 : 1, animSprite.flip_y ? -1 : 1);
        }
        const t = animSprite.tint;
        ctx.globalAlpha = t[3];
        ctx.drawImage(img, sx2, sy2, sw, sh, -screenW / 2, -screenH / 2, screenW, screenH);
        ctx.globalAlpha = 1;
      } else {
        // Fallback: tinted placeholder
        const t = animSprite.tint;
        ctx.fillStyle = `rgba(${Math.round(t[0]*255)},${Math.round(t[1]*255)},${Math.round(t[2]*255)},0.25)`;
        ctx.fillRect(-screenW / 2, -screenH / 2, screenW, screenH);
      }
    } else {
      ctx.fillStyle = "rgba(77,166,255,0.25)";
      ctx.fillRect(-screenW / 2, -screenH / 2, screenW, screenH);
    }
  } else {
    // Regular sprite or placeholder fill
    const color = sprite
      ? `rgba(${Math.round(sprite.color[0]*255)},${Math.round(sprite.color[1]*255)},${Math.round(sprite.color[2]*255)},${(sprite.color[3]*0.5).toFixed(2)})`
      : "rgba(77,166,255,0.25)";
    ctx.fillStyle = color;
    ctx.fillRect(-screenW / 2, -screenH / 2, screenW, screenH);
  }

  // Entity outline
  const outlineColor = isSelected
    ? SELECTED_COLOR
    : sprite ? `rgba(${Math.round(sprite.color[0]*255)},${Math.round(sprite.color[1]*255)},${Math.round(sprite.color[2]*255)},0.9)`
    : ENTITY_COLOR;
  ctx.strokeStyle = outlineColor;
  ctx.lineWidth = isSelected ? 2 : 1;
  ctx.strokeRect(-screenW / 2, -screenH / 2, screenW, screenH);
  ctx.restore();

  if (isSelected && activeTool && activeTool !== "select") {
    drawToolGizmo(ctx, sx, sy, activeTool, cam.zoom);
  }

  // Collider outline (rotates with entity)
  if (collider) {
    const activeCollider = colliderDraft ?? collider;
    const { sx: colCx, sy: colCy } = worldToScreen(
      tx + activeCollider.offset_x,
      ty + activeCollider.offset_y,
      cw, ch,
    );
    const colW = activeCollider.width * cam.zoom;
    const colH = activeCollider.height * cam.zoom;
    const editingCollider = isSelected && activeTool === "collider";

    ctx.save();
    ctx.translate(colCx, colCy);
    ctx.rotate(activeTransform.rotation);

    ctx.strokeStyle = editingCollider ? SELECTED_COLOR : COLLIDER_COLOR;
    ctx.lineWidth = editingCollider ? 1.5 : 1;
    if (!editingCollider) ctx.setLineDash([3, 3]);
    ctx.strokeRect(-colW / 2, -colH / 2, colW, colH);
    ctx.setLineDash([]);

    // Handles at edge midpoints in local space
    if (editingCollider) {
      const handles: [number, number][] = [
        [-colW / 2, 0],
        [colW / 2, 0],
        [0, -colH / 2],
        [0, colH / 2],
      ];
      const hs = 5;
      ctx.fillStyle = SELECTED_COLOR;
      ctx.strokeStyle = BG;
      ctx.lineWidth = 1.5;
      for (const [hx, hy] of handles) {
        ctx.fillRect(hx - hs, hy - hs, hs * 2, hs * 2);
        ctx.strokeRect(hx - hs, hy - hs, hs * 2, hs * 2);
      }
    }
    ctx.restore();
  }

  // Origin dot
  ctx.fillStyle = isSelected ? SELECTED_COLOR : ENTITY_COLOR;
  ctx.beginPath();
  ctx.arc(sx, sy, isSelected ? 3.5 : 2.5, 0, Math.PI * 2);
  ctx.fill();

  // Name label (only when not too zoomed out)
  if (cam.zoom > 0.3) {
    ctx.font = `${Math.min(11, Math.max(9, cam.zoom * 10))}px monospace`;
    ctx.fillStyle = isSelected ? SELECTED_COLOR : LABEL_COLOR;
    ctx.fillText(entity.name, sx + screenW / 2 + 4, sy - screenH / 2 + 10);
  }
}

function drawToolGizmo(ctx: CanvasRenderingContext2D, sx: number, sy: number, tool: ActiveTool, zoom: number) {
  ctx.save();
  ctx.translate(sx, sy);
  ctx.lineWidth = 2;
  if (tool === "move") {
    const len = Math.max(30, Math.min(80, 48 * Math.sqrt(zoom)));
    ctx.strokeStyle = "rgba(232,80,80,0.95)";
    ctx.beginPath(); ctx.moveTo(0, 0); ctx.lineTo(len, 0); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(len, 0); ctx.lineTo(len - 7, -5); ctx.lineTo(len - 7, 5); ctx.closePath(); ctx.fillStyle = ctx.strokeStyle; ctx.fill();
    ctx.strokeStyle = "rgba(80,210,110,0.95)";
    ctx.beginPath(); ctx.moveTo(0, 0); ctx.lineTo(0, len); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(0, len); ctx.lineTo(-5, len - 7); ctx.lineTo(5, len - 7); ctx.closePath(); ctx.fillStyle = ctx.strokeStyle; ctx.fill();
  } else if (tool === "scale") {
    ctx.strokeStyle = SELECTED_COLOR;
    ctx.strokeRect(-8, -8, 16, 16);
    ctx.beginPath(); ctx.moveTo(10, 10); ctx.lineTo(34, 34); ctx.stroke();
    ctx.fillStyle = SELECTED_COLOR;
    ctx.fillRect(30, 30, 8, 8);
  } else if (tool === "rotate") {
    ctx.strokeStyle = SELECTED_COLOR;
    ctx.beginPath(); ctx.arc(0, 0, 28, 0, Math.PI * 1.65); ctx.stroke();
    ctx.fillStyle = SELECTED_COLOR;
    ctx.beginPath(); ctx.moveTo(-7, -28); ctx.lineTo(4, -33); ctx.lineTo(2, -21); ctx.closePath(); ctx.fill();
  }
  ctx.restore();
}

function drawCameraFrame(
  ctx: CanvasRenderingContext2D,
  name: string,
  sx: number,
  sy: number,
  rotation: number,
  cameraZoom: number,
  sceneZoom: number,
  active: boolean,
  selected: boolean,
) {
  const safeZoom = Math.max(cameraZoom, 0.01);
  const frameW = (1280 / safeZoom) * sceneZoom;
  const frameH = (720 / safeZoom) * sceneZoom;
  const color = selected ? SELECTED_COLOR : active ? CAMERA_COLOR : "rgba(91,138,255,0.45)";

  ctx.save();
  ctx.translate(sx, sy);
  ctx.rotate(rotation);
  ctx.strokeStyle = color;
  ctx.lineWidth = selected ? 2 : 1;
  ctx.setLineDash(active ? [] : [6, 4]);
  ctx.fillStyle = active ? "rgba(91,138,255,0.12)" : "rgba(91,138,255,0.05)";
  ctx.fillRect(-frameW / 2, -frameH / 2, frameW, frameH);
  ctx.strokeRect(-frameW / 2, -frameH / 2, frameW, frameH);
  ctx.setLineDash([]);

  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.moveTo(0, -8);
  ctx.lineTo(9, 8);
  ctx.lineTo(-9, 8);
  ctx.closePath();
  ctx.stroke();

  ctx.font = "10px monospace";
  ctx.fillText(active ? `${name} · active` : name, -frameW / 2 + 6, -frameH / 2 + 14);
  ctx.restore();
}

// ─── Game tab ───────────────────────────────────────────────────────────────

const ENGINE_URL = "http://127.0.0.1:7878";

const WS_URL = "ws://127.0.0.1:7878/stream";
const STREAM_W = 960;
const STREAM_H = 540;

function GameView({ engineReady, isPlaying }: { engineReady: boolean; isPlaying: boolean }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const heldKeys = useRef<Set<string>>(new Set());
  const [fps, setFps] = useState(0);
  const [connected, setConnected] = useState(false);

  // WebSocket raw-RGBA frame streaming
  useEffect(() => {
    if (!engineReady) return;

    let ws: WebSocket | null = null;
    let stopped = false;
    const fpsState = { frames: 0, last: performance.now() };

    const connect = () => {
      if (stopped) return;
      ws = new WebSocket(WS_URL);
      ws.binaryType = "arraybuffer";
      ws.onopen = () => setConnected(true);
      ws.onclose = () => {
        setConnected(false);
        if (!stopped) setTimeout(connect, 1000);
      };
      ws.onerror = () => ws?.close();
      ws.onmessage = (e: MessageEvent<ArrayBuffer>) => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;
        const rgba = new Uint8ClampedArray(e.data);
        ctx.putImageData(new ImageData(rgba, STREAM_W, STREAM_H), 0, 0);
        fpsState.frames++;
        const now = performance.now();
        if (now - fpsState.last >= 1000) {
          setFps(fpsState.frames);
          fpsState.frames = 0;
          fpsState.last = now;
        }
      };
    };
    connect();

    return () => {
      stopped = true;
      ws?.close();
      ws = null;
    };
  }, [engineReady]);

  // Forward keyboard input to engine
  useEffect(() => {
    if (!engineReady) return;
    const sendKeys = () =>
      fetch(`${ENGINE_URL}/input/keys`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ keys: [...heldKeys.current] }),
      }).catch(() => {});
    const onKeyDown = (e: KeyboardEvent) => { if (!e.repeat) { heldKeys.current.add(e.key); sendKeys(); } };
    const onKeyUp = (e: KeyboardEvent) => { heldKeys.current.delete(e.key); sendKeys(); };
    const onBlur = () => { heldKeys.current.clear(); sendKeys(); };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", onBlur);
      heldKeys.current.clear();
      sendKeys();
    };
  }, [engineReady]);

  return (
    <div style={{ width: "100%", height: "100%", position: "relative", background: "#000" }}>
      {engineReady ? (
        <canvas
          ref={canvasRef}
          width={STREAM_W}
          height={STREAM_H}
          style={{ display: "block", width: "100%", height: "100%", objectFit: "contain" }}
        />
      ) : (
        <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <span style={{ fontSize: "11px", color: "var(--ink-4)", fontFamily: "var(--font-mono)" }}>waiting for engine…</span>
        </div>
      )}

      {/* Status strip */}
      {engineReady && (
        <div style={{
          position: "absolute", bottom: "10px", right: "12px",
          display: "flex", alignItems: "center", gap: "10px",
          fontFamily: "var(--font-mono)", fontSize: "10px",
        }}>
          <span style={{ color: "var(--ink-4)" }}>{fps} fps</span>
          <span style={{
            display: "flex", alignItems: "center", gap: "4px",
            color: isPlaying ? "var(--moss)" : "var(--ink-4)",
          }}>
            <span style={{
              width: "5px", height: "5px",
              background: connected ? (isPlaying ? "var(--moss)" : "var(--ink-4)") : "var(--amber)",
              display: "inline-block", flexShrink: 0,
            }} />
            {!connected ? "reconnecting…" : isPlaying ? "live" : "paused"}
          </span>
        </div>
      )}
    </div>
  );
}
