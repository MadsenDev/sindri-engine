import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { decodeRawFrame } from "../app/frameDecoder";
import { presentImageData } from "../app/framePresenter";
import Gizmo from "./Gizmo";
import { Tool } from "./Toolbar";
import type { EntityInfo, CameraInfo } from "../app/types";

interface TransformData {
  position: [number, number];
  rotation: number;
  scale: [number, number];
}

interface TransformHierarchyData {
  local_position: [number, number];
  local_rotation: number;
  local_scale: [number, number];
  world_position: [number, number];
  world_rotation: number;
  world_scale: [number, number];
  parent_world_position: [number, number];
  parent_world_rotation: number;
  parent_world_scale: [number, number];
}

interface TransformHierarchyEntry {
  entity_id: number;
  transform: TransformHierarchyData;
}

interface ViewportProps {
  entities: EntityInfo[];
  selectedEntityId: number | null;
  selectedEntityIds?: number[];
  onEntityClick?: (entityId: number) => void;
  onContextMenuOpen?: (screen: { x: number; y: number }, world: { x: number; y: number }) => void;
  onSelectionChange?: (ids: number[]) => void;
  onTransformChange?: () => void;
  onAssetDrop?: (asset: { path: string; kind: string }, world: { x: number; y: number }) => void;
  isPlaying?: boolean;
  tool: Tool;
}

export interface ViewportHandle {
  frameSelection: () => void;
  resetCamera: () => void;
  setGridSize: (size: number) => void;
}

const Viewport = forwardRef<ViewportHandle, ViewportProps>(({
  entities,
  selectedEntityId,
  selectedEntityIds,
  onEntityClick,
  onContextMenuOpen,
  onSelectionChange,
  onTransformChange,
  onAssetDrop,
  isPlaying = false,
  tool,
}, ref) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [camera, setCamera] = useState({ x: 0, y: 0, zoom: 1 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [transformCache, setTransformCache] = useState<Map<number, TransformData>>(new Map());
  const [canvasSize, setCanvasSize] = useState({ width: 0, height: 0 });
  const renderInFlight = useRef(false);
  const renderQueued = useRef(false);
  const overlayQueued = useRef(false);
  const lastFrameRef = useRef<ImageData | null>(null);
  const [sceneRenderNonce, setSceneRenderNonce] = useState(0);
  const [overlayRenderNonce, setOverlayRenderNonce] = useState(0);
  const [gridSize, setGridSize] = useState(50);
  const [cameraEntities, setCameraEntities] = useState<CameraInfo[]>([]);
  const [boxSelect, setBoxSelect] = useState<{
    start: { x: number; y: number };
    end: { x: number; y: number };
    additive: boolean;
  } | null>(null);

  useImperativeHandle(ref, () => ({
    frameSelection: () => {
      const ids =
        selectedEntityIds && selectedEntityIds.length > 0
          ? selectedEntityIds
          : selectedEntityId !== null
          ? [selectedEntityId]
          : [];
      if (ids.length === 0) {
        return;
      }

      let minX = Infinity;
      let minY = Infinity;
      let maxX = -Infinity;
      let maxY = -Infinity;
      const baseHalf = 16;

      for (const id of ids) {
        const transform = transformCache.get(id);
        if (!transform) continue;
        const halfW = Math.max(baseHalf, baseHalf * transform.scale[0]);
        const halfH = Math.max(baseHalf, baseHalf * transform.scale[1]);
        minX = Math.min(minX, transform.position[0] - halfW);
        minY = Math.min(minY, transform.position[1] - halfH);
        maxX = Math.max(maxX, transform.position[0] + halfW);
        maxY = Math.max(maxY, transform.position[1] + halfH);
      }

      if (!isFinite(minX) || !isFinite(minY) || !isFinite(maxX) || !isFinite(maxY)) {
        return;
      }

      const centerX = (minX + maxX) / 2;
      const centerY = (minY + maxY) / 2;
      const boundsW = Math.max(1, maxX - minX);
      const boundsH = Math.max(1, maxY - minY);
      const viewW = canvasSize.width || canvasRef.current?.width || 0;
      const viewH = canvasSize.height || canvasRef.current?.height || 0;
      if (viewW === 0 || viewH === 0) {
        return;
      }
      const fitZoom = Math.min(viewW / boundsW, viewH / boundsH) * 0.8;
      setCamera({
        x: centerX,
        y: centerY,
        zoom: Math.max(0.1, Math.min(5, fitZoom)),
      });
    },
    resetCamera: () => {
      setCamera({ x: 0, y: 0, zoom: 1 });
    },
    setGridSize: (size: number) => {
      setGridSize(Math.max(5, size));
    },
  }));

  // Cache transforms when entities change (not every frame)
  useEffect(() => {
    const fetchData = async () => {
      const transforms = new Map<number, TransformData>();
      const data = await invoke<TransformHierarchyEntry[]>("transforms_list");
      const allowed = new Set(entities.filter((e) => e.has_transform).map((e) => e.id));
      for (const entry of data) {
        if (!allowed.has(entry.entity_id)) continue;
        transforms.set(entry.entity_id, {
          position: entry.transform.world_position,
          rotation: entry.transform.world_rotation,
          scale: entry.transform.world_scale,
        });
      }
      setTransformCache(transforms);
    };

    fetchData().catch((error) => {
      console.error("Failed to fetch transforms", error);
    });
  }, [entities]);

  useEffect(() => {
    let active = true;
    const fetchCameras = async () => {
      try {
        const cameras = await invoke<CameraInfo[]>("camera_entities");
        if (active) {
          setCameraEntities(cameras);
        }
      } catch {
        // Ignore transient IPC errors.
      }
    };

    fetchCameras();
    const interval = window.setInterval(fetchCameras, isPlaying ? 1000 : 2500);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [entities, isPlaying]);

  const requestSceneRender = () => {
    if (renderQueued.current) return;
    renderQueued.current = true;
    requestAnimationFrame(() => {
      renderQueued.current = false;
      setSceneRenderNonce((prev) => prev + 1);
    });
  };

  const requestOverlayRender = () => {
    if (overlayQueued.current) return;
    overlayQueued.current = true;
    requestAnimationFrame(() => {
      overlayQueued.current = false;
      setOverlayRenderNonce((prev) => prev + 1);
    });
  };

  useEffect(() => {
    requestSceneRender();
  }, [
    camera,
    transformCache,
    canvasSize.width,
    canvasSize.height,
  ]);

  useEffect(() => {
    requestOverlayRender();
  }, [
    camera,
    gridSize,
    transformCache,
    selectedEntityId,
    selectedEntityIds,
    boxSelect,
    cameraEntities,
  ]);

  // On-demand rendering (engine/wgpu offscreen)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const width = canvas.width;
    const height = canvas.height;

    const drawGridOverlay = () => {
      const size = gridSize * camera.zoom;
      if (size < 4) {
        return;
      }
      const originX = (-camera.x) * camera.zoom + width / 2;
      const originY = (-camera.y) * camera.zoom + height / 2;
      const startX = ((originX % size) + size) % size;
      const startY = ((originY % size) + size) % size;

      ctx.strokeStyle = "rgba(255, 255, 255, 0.06)";
      ctx.lineWidth = 1;

      for (let x = startX; x < width; x += size) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
      }

      for (let y = startY; y < height; y += size) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
      }
    };

    const drawSelectionOverlay = () => {
      const ids =
        selectedEntityIds && selectedEntityIds.length > 0
          ? selectedEntityIds
          : selectedEntityId !== null
          ? [selectedEntityId]
          : [];
      if (ids.length === 0 && !boxSelect) {
        return;
      }

      const halfDefault = 16;
      let minX = Infinity;
      let minY = Infinity;
      let maxX = -Infinity;
      let maxY = -Infinity;

      for (const id of ids) {
        const transform = transformCache.get(id);
        if (!transform) continue;
        const screenX =
          (transform.position[0] - camera.x) * camera.zoom + width / 2;
        const screenY =
          (transform.position[1] - camera.y) * camera.zoom + height / 2;
        const halfW = Math.max(
          halfDefault,
          halfDefault * transform.scale[0]
        ) * camera.zoom;
        const halfH = Math.max(
          halfDefault,
          halfDefault * transform.scale[1]
        ) * camera.zoom;
        minX = Math.min(minX, screenX - halfW);
        minY = Math.min(minY, screenY - halfH);
        maxX = Math.max(maxX, screenX + halfW);
        maxY = Math.max(maxY, screenY + halfH);
      }

      if (minX !== Infinity) {
        ctx.strokeStyle = "#60a5fa";
        ctx.lineWidth = 2;
        ctx.setLineDash([6, 4]);
        ctx.strokeRect(minX, minY, maxX - minX, maxY - minY);
        ctx.setLineDash([]);
      }

      if (boxSelect) {
        const startX = boxSelect.start.x;
        const startY = boxSelect.start.y;
        const endX = boxSelect.end.x;
        const endY = boxSelect.end.y;
        const rectX = Math.min(startX, endX);
        const rectY = Math.min(startY, endY);
        const rectW = Math.abs(endX - startX);
        const rectH = Math.abs(endY - startY);
        ctx.fillStyle = "rgba(96, 165, 250, 0.15)";
        ctx.strokeStyle = "#60a5fa";
        ctx.lineWidth = 1;
        ctx.setLineDash([4, 4]);
        ctx.fillRect(rectX, rectY, rectW, rectH);
        ctx.strokeRect(rectX, rectY, rectW, rectH);
        ctx.setLineDash([]);
      }
    };

    const drawCameraOverlay = () => {
      if (cameraEntities.length === 0) {
        return;
      }

      for (const cam of cameraEntities) {
        const centerX = cam.world_position[0] + cam.offset[0];
        const centerY = cam.world_position[1] + cam.offset[1];
        const halfW = width / (2 * cam.zoom);
        const halfH = height / (2 * cam.zoom);
        const left = centerX - halfW;
        const right = centerX + halfW;
        const top = centerY - halfH;
        const bottom = centerY + halfH;

        const leftScreen = (left - camera.x) * camera.zoom + width / 2;
        const rightScreen = (right - camera.x) * camera.zoom + width / 2;
        const topScreen = (top - camera.y) * camera.zoom + height / 2;
        const bottomScreen = (bottom - camera.y) * camera.zoom + height / 2;

        ctx.save();
        ctx.strokeStyle = cam.active ? "rgba(80, 220, 140, 0.9)" : "rgba(160, 160, 160, 0.7)";
        ctx.lineWidth = cam.active ? 2 : 1;
        if (!cam.active) {
          ctx.setLineDash([6, 4]);
        }
        ctx.strokeRect(
          leftScreen,
          topScreen,
          rightScreen - leftScreen,
          bottomScreen - topScreen
        );
        ctx.setLineDash([]);

        const centerScreenX = (centerX - camera.x) * camera.zoom + width / 2;
        const centerScreenY = (centerY - camera.y) * camera.zoom + height / 2;
        const baseScreenX =
          (cam.world_position[0] - camera.x) * camera.zoom + width / 2;
        const baseScreenY =
          (cam.world_position[1] - camera.y) * camera.zoom + height / 2;

        ctx.strokeStyle = cam.active ? "rgba(80, 220, 140, 0.9)" : "rgba(160, 160, 160, 0.7)";
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(baseScreenX, baseScreenY);
        ctx.lineTo(centerScreenX, centerScreenY);
        ctx.stroke();

        const forwardLen = 60;
        const fx = Math.cos(cam.rotation) * forwardLen;
        const fy = Math.sin(cam.rotation) * forwardLen;
        const forwardScreenX =
          (centerX + fx - camera.x) * camera.zoom + width / 2;
        const forwardScreenY =
          (centerY + fy - camera.y) * camera.zoom + height / 2;
        ctx.beginPath();
        ctx.moveTo(centerScreenX, centerScreenY);
        ctx.lineTo(forwardScreenX, forwardScreenY);
        ctx.stroke();

        const safeInset = 0.1;
        const safeLeft = centerX - halfW * (1 - safeInset);
        const safeRight = centerX + halfW * (1 - safeInset);
        const safeTop = centerY - halfH * (1 - safeInset);
        const safeBottom = centerY + halfH * (1 - safeInset);
        const safeLeftScreen = (safeLeft - camera.x) * camera.zoom + width / 2;
        const safeRightScreen = (safeRight - camera.x) * camera.zoom + width / 2;
        const safeTopScreen = (safeTop - camera.y) * camera.zoom + height / 2;
        const safeBottomScreen = (safeBottom - camera.y) * camera.zoom + height / 2;
        ctx.setLineDash([4, 3]);
        ctx.strokeRect(
          safeLeftScreen,
          safeTopScreen,
          safeRightScreen - safeLeftScreen,
          safeBottomScreen - safeTopScreen
        );
        ctx.setLineDash([]);

        ctx.fillStyle = cam.active ? "#55f1b0" : "#c0c0c0";
        ctx.font = "11px monospace";
        ctx.fillText(`Cam ${cam.entity_id}`, leftScreen + 6, topScreen - 6);
        ctx.restore();
      }
    };

    const drawCachedFrame = async () => {
      const cached = lastFrameRef.current;
      if (cached) {
        await presentImageData(canvas, cached);
      } else {
        ctx.clearRect(0, 0, width, height);
      }
      drawGridOverlay();
      drawCameraOverlay();
      drawSelectionOverlay();
    };

    const renderFrame = async () => {
      if (overlayRenderNonce > 0 || sceneRenderNonce > 0) {
        await drawCachedFrame();
      }

      if (sceneRenderNonce === 0) {
        return;
      }

      if (renderInFlight.current) return;
      if (width === 0 || height === 0) return;

      renderInFlight.current = true;
      try {
        const frame = await invoke<Uint8Array>("viewport_render_raw", {
          width,
          height,
          cameraX: camera.x,
          cameraY: camera.y,
          zoom: camera.zoom,
          rotation: 0,
        });

        const image = decodeRawFrame(frame, width, height);
        lastFrameRef.current = image;
        await drawCachedFrame();
      } catch (error) {
        console.error("Viewport render failed:", error);
      } finally {
        renderInFlight.current = false;
      }
    };

    renderFrame().catch((error) => {
      console.error("Viewport frame pipeline failed:", error);
    });
  }, [
    sceneRenderNonce,
    overlayRenderNonce,
    camera,
    cameraEntities,
    gridSize,
    selectedEntityId,
    selectedEntityIds,
    boxSelect,
    transformCache,
    canvasSize.width,
    canvasSize.height,
  ]);

  const screenToWorld = (clientX: number, clientY: number) => {
    const canvas = canvasRef.current;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    return {
      screen: { x, y },
      world: {
        x: (x - canvas.width / 2) / camera.zoom + camera.x,
        y: (y - canvas.height / 2) / camera.zoom + camera.y,
      },
    };
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    // Don't interfere with gizmo - let it handle clicks on selected entities
    // The gizmo canvas is on top and will capture the click
    
    if (e.button === 1 || (e.button === 0 && e.altKey)) {
      // Middle mouse or Alt+Left = pan
      setIsDragging(true);
      setDragStart({ x: e.clientX, y: e.clientY });
    } else if (e.button === 0) {
      // Left click = select entity or start box select
      const canvas = canvasRef.current;
      if (!canvas) return;

      const converted = screenToWorld(e.clientX, e.clientY);
      if (!converted) return;
      const { screen, world } = converted;
      const x = screen.x;
      const y = screen.y;
      const worldX = world.x;
      const worldY = world.y;

      // Find closest entity using cached transforms
      let closestEntity: { id: number; dist: number } | null = null;
      
      for (const entity of entities) {
        if (!entity.has_transform) continue;
        const transform = transformCache.get(entity.id);
        if (!transform) continue;

        const dx = transform.position[0] - worldX;
        const dy = transform.position[1] - worldY;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (dist < 20 && (!closestEntity || dist < closestEntity.dist)) {
          closestEntity = { id: entity.id, dist };
        }
      }

      if (closestEntity && onEntityClick) {
        onEntityClick(closestEntity.id);
      } else {
        setBoxSelect({
          start: { x, y },
          end: { x, y },
          additive: e.ctrlKey || e.metaKey,
        });
      }
    }
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (isDragging) {
      const dx = (e.clientX - dragStart.x) / camera.zoom;
      const dy = (e.clientY - dragStart.y) / camera.zoom;
      setCamera((prev) => ({
        ...prev,
        x: prev.x - dx,
        y: prev.y - dy,
      }));
      setDragStart({ x: e.clientX, y: e.clientY });
    } else if (boxSelect) {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (!rect) return;
      setBoxSelect((prev) =>
        prev
          ? {
              ...prev,
              end: { x: e.clientX - rect.left, y: e.clientY - rect.top },
            }
          : prev
      );
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
    if (!boxSelect) return;

    const canvas = canvasRef.current;
    if (!canvas) {
      setBoxSelect(null);
      return;
    }

    const startX = Math.min(boxSelect.start.x, boxSelect.end.x);
    const endX = Math.max(boxSelect.start.x, boxSelect.end.x);
    const startY = Math.min(boxSelect.start.y, boxSelect.end.y);
    const endY = Math.max(boxSelect.start.y, boxSelect.end.y);

    const worldMinX = (startX - canvas.width / 2) / camera.zoom + camera.x;
    const worldMaxX = (endX - canvas.width / 2) / camera.zoom + camera.x;
    const worldMinY = (startY - canvas.height / 2) / camera.zoom + camera.y;
    const worldMaxY = (endY - canvas.height / 2) / camera.zoom + camera.y;

    const selected: number[] = [];
    for (const entity of entities) {
      if (!entity.has_transform) continue;
      const transform = transformCache.get(entity.id);
      if (!transform) continue;
      const px = transform.position[0];
      const py = transform.position[1];
      if (px >= worldMinX && px <= worldMaxX && py >= worldMinY && py <= worldMaxY) {
        selected.push(entity.id);
      }
    }

    if (onSelectionChange) {
      if (boxSelect.additive) {
        const base =
          selectedEntityIds && selectedEntityIds.length > 0
            ? selectedEntityIds
            : selectedEntityId !== null
            ? [selectedEntityId]
            : [];
        const merged = Array.from(new Set([...base, ...selected]));
        onSelectionChange(merged);
      } else {
        onSelectionChange(selected);
      }
    }
    setBoxSelect(null);
  };

  const handleWheel = (e: React.WheelEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    setCamera((prev) => ({
      ...prev,
      zoom: Math.max(0.1, Math.min(5, prev.zoom * delta)),
    }));
  };

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const resize = () => {
      // Get the actual display size (CSS pixels)
      const displayWidth = canvas.clientWidth;
      const displayHeight = canvas.clientHeight;

      if (canvas.width !== displayWidth || canvas.height !== displayHeight) {
        canvas.width = displayWidth;
        canvas.height = displayHeight;
        setCanvasSize({ width: displayWidth, height: displayHeight });
      }
    };

    resize();
    
    // Use ResizeObserver to detect when container size changes
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      resizeObserver.disconnect();
    };
  }, []);

  return (
    <div className="relative w-full h-full">
      <canvas
        ref={canvasRef}
        className="w-full h-full cursor-crosshair"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onContextMenu={(e) => {
          if (!onContextMenuOpen) return;
          e.preventDefault();
          const converted = screenToWorld(e.clientX, e.clientY);
          if (!converted) return;
          onContextMenuOpen(
            { x: e.clientX, y: e.clientY },
            converted.world
          );
        }}
        onDragOver={(e) => {
          if (!onAssetDrop) return;
          e.preventDefault();
          e.dataTransfer.dropEffect = "copy";
        }}
        onDrop={(e) => {
          if (!onAssetDrop) return;
          e.preventDefault();
          const raw = e.dataTransfer.getData("application/x-forge2d-asset");
          if (!raw) return;
          const converted = screenToWorld(e.clientX, e.clientY);
          if (!converted) return;
          try {
            const asset = JSON.parse(raw) as { path: string; kind: string };
            onAssetDrop(asset, converted.world);
          } catch (error) {
            console.error("Invalid asset drop payload", error);
          }
        }}
        onWheel={handleWheel}
        style={{ display: "block" }}
      />
      {!isPlaying && (
        <Gizmo
          entityId={selectedEntityId}
          camera={camera}
          canvasWidth={canvasSize.width}
          canvasHeight={canvasSize.height}
          tool={tool}
          onTransformUpdate={(transform) => {
            // Update cache immediately during drag for smooth rendering
            if (selectedEntityId !== null) {
              setTransformCache((prev) => {
                const next = new Map(prev);
                next.set(selectedEntityId, transform);
                return next;
              });
            }
          }}
          onTransformChange={async () => {
            // Refresh full entity list when drag ends
            if (onTransformChange) {
              onTransformChange();
            }
          }}
        />
      )}
    </div>
  );
});

export default Viewport;
