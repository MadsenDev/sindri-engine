import { useRef, useEffect, useState, useCallback } from "react";
import type { ActiveTool, Scene, Entity } from "../App";

interface Props {
  scene: Scene | null;
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  activeTool: ActiveTool;
  onTransformCommit: (change: TransformChange) => void;
  engineReady: boolean;
  isPlaying: boolean;
}

interface Camera {
  x: number;   // world-space center
  y: number;
  zoom: number; // pixels per world unit
}

const BG = "#0a0b0d";
const GRID_MINOR = "rgba(255,255,255,0.035)";
const GRID_MAJOR = "rgba(255,255,255,0.075)";
const AXIS_COLOR = "rgba(255,255,255,0.15)";
const ENTITY_COLOR = "#4da6ff";
const SELECTED_COLOR = "#e8a838";
const COLLIDER_COLOR = "rgba(80,230,100,0.35)";
const CAMERA_COLOR = "#5b8aff";
const LABEL_COLOR = "#8a9bb0";

export default function Viewport({ scene, selectedId, onSelect, activeTool, onTransformCommit, engineReady, isPlaying }: Props) {
  const [tab, setTab] = useState<"scene" | "game">("scene");

  // Auto-switch to game tab when play starts, back to scene when stopped
  useEffect(() => {
    if (isPlaying) setTab("game");
  }, [isPlaying]);

  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", minHeight: 0 }}>
      {/* Tab bar */}
      <div style={{
        height: "30px", background: "var(--bg-2)",
        borderBottom: "1px solid var(--border)",
        display: "flex", alignItems: "center",
        padding: "0 8px", gap: "2px", flexShrink: 0,
      }}>
        {(["scene", "game"] as const).map(t => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: "4px 10px", fontSize: "11px",
            color: tab === t ? "var(--text-bright)" : "var(--text-muted)",
            background: tab === t ? "var(--bg-4)" : "transparent",
            border: "none", borderRadius: "var(--radius) var(--radius) 0 0",
            cursor: "pointer", textTransform: "capitalize",
          }}>{t}</button>
        ))}
        <div style={{ flex: 1 }} />
        {isPlaying && tab === "game" && (
          <span style={{ fontSize: "9px", color: "rgb(80,210,110)", letterSpacing: "0.08em", display: "flex", alignItems: "center", gap: "4px" }}>
            <span style={{ width: "5px", height: "5px", borderRadius: "50%", background: "rgb(80,210,110)", animation: "pulse 1.5s infinite", display: "inline-block" }} />
            PLAYING
          </span>
        )}
        <span style={{ fontSize: "10px", color: "var(--text-dim)" }}>1280×720</span>
      </div>

      <div style={{ flex: 1, position: "relative", overflow: "hidden" }}>
        {tab === "scene" ? (
          <SceneView
            scene={scene}
            selectedId={selectedId}
            onSelect={onSelect}
            activeTool={activeTool}
            onTransformCommit={onTransformCommit}
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
  tool: Exclude<ActiveTool, "select">;
  startMouse: { x: number; y: number };
  startTransform: TransformDraft;
  startAngle: number;
}

function SceneView({
  scene,
  selectedId,
  onSelect,
  activeTool,
  onTransformCommit,
}: {
  scene: Scene | null;
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  activeTool: ActiveTool;
  onTransformCommit: (change: TransformChange) => void;
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
  const dragRef = useRef<DragState | null>(null);
  const draftRef = useRef<Map<number, TransformDraft>>(new Map());

  useEffect(() => { sceneRef.current = scene; }, [scene]);
  useEffect(() => { selectedRef.current = selectedId; }, [selectedId]);
  useEffect(() => { activeToolRef.current = activeTool; }, [activeTool]);

  const worldToScreen = (wx: number, wy: number, cw: number, ch: number) => ({
    sx: (wx - cameraRef.current.x) * cameraRef.current.zoom + cw / 2,
    sy: (wy - cameraRef.current.y) * cameraRef.current.zoom + ch / 2,
  });

  const screenToWorld = (sx: number, sy: number, cw: number, ch: number) => ({
    wx: (sx - cw / 2) / cameraRef.current.zoom + cameraRef.current.x,
    wy: (sy - ch / 2) / cameraRef.current.zoom + cameraRef.current.y,
  });

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

    ctx.fillStyle = BG;
    ctx.fillRect(0, 0, cw, ch);

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

    // — Entities —
    if (sc) {
      for (const entity of Object.values(sc.entities)) {
        drawEntity(ctx, entity, selId, cw, ch, cam, worldToScreen, draftRef.current.get(entity.id), activeToolRef.current);
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
    const hit = hitTest(sc, wx, wy);

    if (hit !== null && hit !== selectedRef.current) {
      onSelect(hit);
    }

    const tool = activeToolRef.current;
    const targetId = hit ?? selectedRef.current;
    if (tool === "select" || targetId === null) {
      return;
    }

    const entity = sc.entities[String(targetId)];
    const transform = entity ? getTransform(entity) : null;
    if (!transform) {
      return;
    }

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

  return (
    <div ref={containerRef} style={{ width: "100%", height: "100%", cursor: activeTool === "select" ? "crosshair" : "grab" }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
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

function hitTest(scene: Scene, wx: number, wy: number): number | null {
  let hit: number | null = null;
  for (const entity of Object.values(scene.entities)) {
    const transform = getTransform(entity);
    const sprite = entity.components.find(c => c.type === "Sprite") as { type: "Sprite"; width: number; height: number } | undefined;
    const camera = entity.components.find(c => c.type === "Camera") as { type: "Camera"; zoom: number } | undefined;
    if (!transform) continue;
    const hw = sprite ? Math.abs(sprite.width * transform.scale_x) * 0.5 : camera ? 12 : Math.max(Math.abs(transform.scale_x) * 16, 12);
    const hh = sprite ? Math.abs(sprite.height * transform.scale_y) * 0.5 : camera ? 12 : Math.max(Math.abs(transform.scale_y) * 16, 12);
    if (wx >= transform.x - hw && wx <= transform.x + hw && wy >= transform.y - hh && wy <= transform.y + hh) {
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
  selectedId: number | null,
  cw: number,
  ch: number,
  cam: Camera,
  worldToScreen: (wx: number, wy: number, cw: number, ch: number) => { sx: number; sy: number },
  draft?: TransformDraft,
  activeTool?: ActiveTool,
) {
  const transform = entity.components.find(c => c.type === "Transform") as
    | { type: "Transform"; x: number; y: number; scale_x: number; scale_y: number; rotation: number }
    | undefined;
  const sprite = entity.components.find(c => c.type === "Sprite") as
    | { type: "Sprite"; width: number; height: number; color: [number, number, number, number] }
    | undefined;
  const collider = entity.components.find(c => c.type === "Collider") as
    | { type: "Collider"; width: number; height: number; offset_x: number; offset_y: number }
    | undefined;
  const cameraComp = entity.components.find(c => c.type === "Camera") as
    | { type: "Camera"; active?: boolean; zoom: number }
    | undefined;

  const isSelected = entity.id === selectedId;
  const activeTransform = draft ?? transform;
  const tx = activeTransform?.x ?? 0;
  const ty = activeTransform?.y ?? 0;
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
    if (!sprite && !collider) return;
  }

  const hw = sprite ? sprite.width * activeTransform.scale_x * 0.5 : Math.max(activeTransform.scale_x * 16, 1);
  const hh = sprite ? sprite.height * activeTransform.scale_y * 0.5 : Math.max(activeTransform.scale_y * 16, 1);

  const screenW = hw * 2 * cam.zoom;
  const screenH = hh * 2 * cam.zoom;

  const color = sprite ? `rgba(${Math.round(sprite.color[0] * 255)},${Math.round(sprite.color[1] * 255)},${Math.round(sprite.color[2] * 255)},${(sprite.color[3] * 0.5).toFixed(2)})` : "rgba(77,166,255,0.25)";
  ctx.save();
  ctx.translate(sx, sy);
  ctx.rotate(activeTransform.rotation);

  // Entity fill
  ctx.fillStyle = color;
  ctx.fillRect(-screenW / 2, -screenH / 2, screenW, screenH);

  // Entity outline
  ctx.strokeStyle = isSelected ? SELECTED_COLOR : (sprite ? `rgba(${Math.round(sprite.color[0] * 255)},${Math.round(sprite.color[1] * 255)},${Math.round(sprite.color[2] * 255)},0.9)` : ENTITY_COLOR);
  ctx.lineWidth = isSelected ? 2 : 1;
  ctx.strokeRect(-screenW / 2, -screenH / 2, screenW, screenH);
  ctx.restore();

  if (isSelected && activeTool && activeTool !== "select") {
    drawToolGizmo(ctx, sx, sy, activeTool, cam.zoom);
  }

  // Collider outline
  if (collider) {
    const { sx: colSx, sy: colSy } = worldToScreen(tx + collider.offset_x - collider.width * 0.5, ty + collider.offset_y - collider.height * 0.5, cw, ch);
    const colW = collider.width * cam.zoom;
    const colH = collider.height * cam.zoom;
    ctx.strokeStyle = COLLIDER_COLOR;
    ctx.lineWidth = 1;
    ctx.setLineDash([3, 3]);
    ctx.strokeRect(colSx, colSy, colW, colH);
    ctx.setLineDash([]);
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

function GameView({ engineReady, isPlaying }: { engineReady: boolean; isPlaying: boolean }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const heldKeys = useRef<Set<string>>(new Set());

  // Capture keyboard when game view is focused and forward to engine
  useEffect(() => {
    if (!engineReady) return;

    const sendKeys = () => {
      fetch(`${ENGINE_URL}/input/keys`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ keys: [...heldKeys.current] }),
      }).catch(() => {});
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.repeat) return;
      heldKeys.current.add(e.key);
      sendKeys();
    };
    const onKeyUp = (e: KeyboardEvent) => {
      heldKeys.current.delete(e.key);
      sendKeys();
    };
    const onBlur = () => {
      heldKeys.current.clear();
      sendKeys();
    };

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", onBlur);
      // Clear keys on unmount
      heldKeys.current.clear();
      sendKeys();
    };
  }, [engineReady]);

  return (
    <div
      ref={containerRef}
      tabIndex={0}
      style={{ width: "100%", height: "100%", position: "relative", background: "#000", outline: "none", display: "flex", alignItems: "center", justifyContent: "center" }}
    >
      <span style={{ fontSize: "11px", color: engineReady ? "var(--text-muted)" : "var(--text-dim)" }}>
        {engineReady ? "native play window" : "waiting for engine…"}
      </span>

      {/* Play state indicator */}
      <div style={{
        position: "absolute", top: "8px", right: "8px",
        display: "flex", alignItems: "center", gap: "5px",
        background: isPlaying ? "rgba(60,200,100,0.1)" : "var(--bg-2)",
        border: `1px solid ${isPlaying ? "rgba(60,200,100,0.3)" : "var(--border)"}`,
        borderRadius: "var(--radius)", padding: "3px 8px",
        fontSize: "10px",
        color: isPlaying ? "rgb(80,210,110)" : "var(--text-dim)",
        transition: "all 0.2s",
      }}>
        <span style={{
          width: "5px", height: "5px", borderRadius: "50%",
          background: isPlaying ? "rgb(80,210,110)" : "var(--text-dim)",
          animation: isPlaying ? "pulse 1.5s infinite" : "none",
          display: "inline-block",
          flexShrink: 0,
        }} />
        {isPlaying ? "LIVE" : "PAUSED"}
      </div>
    </div>
  );
}
