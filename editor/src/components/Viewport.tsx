import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Gizmo from "./Gizmo";
import { Tool } from "./Toolbar";

interface EntityInfo {
  id: number;
  has_transform: boolean;
  has_sprite: boolean;
  has_physics: boolean;
  has_camera: boolean;
  parent_id: number | null;
  children: number[];
}

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

interface CameraInfo {
  entity_id: number;
  world_position: [number, number];
  rotation: number;
  zoom: number;
  offset: [number, number];
  active: boolean;
}

interface ViewportFrame {
  width: number;
  height: number;
  rgba: number[];
}

interface ViewportProps {
  entities: EntityInfo[];
  selectedEntityId: number | null;
  selectedEntityIds?: number[];
  onEntityClick?: (entityId: number) => void;
  onContextMenuOpen?: (screen: { x: number; y: number }, world: { x: number; y: number }) => void;
  onSelectionChange?: (ids: number[]) => void;
  onTransformChange?: () => void;
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
  const [renderNonce, setRenderNonce] = useState(0);
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
      
      const promises = entities
        .filter((e) => e.has_transform)
        .map(async (entity) => {
          const transform = await invoke<TransformHierarchyData | null>(
            "transform_get_hierarchy",
            {
              entityId: entity.id,
            }
          );
          if (transform) {
            transforms.set(entity.id, {
              position: transform.world_position,
              rotation: transform.world_rotation,
              scale: transform.world_scale,
            });
          }
        });
      await Promise.all(promises);
      setTransformCache(transforms);
    };

    fetchData();
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
    const interval = window.setInterval(fetchCameras, 1000);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [entities]);

  const requestRender = () => {
    setRenderNonce((prev) => prev + 1);
  };

  useEffect(() => {
    requestRender();
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

    const renderFrame = async () => {
      if (renderInFlight.current) return;
      const width = canvas.width;
      const height = canvas.height;
      if (width === 0 || height === 0) return;

      renderInFlight.current = true;
      try {
        const frame = await invoke<ViewportFrame>("viewport_render", {
          width,
          height,
          cameraX: camera.x,
          cameraY: camera.y,
          zoom: camera.zoom,
          rotation: 0,
        });

        const image = new ImageData(
          new Uint8ClampedArray(frame.rgba),
          frame.width,
          frame.height
        );
        ctx.putImageData(image, 0, 0);
        drawGridOverlay();
        drawCameraOverlay();
        drawSelectionOverlay();
      } catch (error) {
        console.error("Viewport render failed:", error);
      } finally {
        renderInFlight.current = false;
      }
    };

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

    renderFrame();
  }, [renderNonce, camera, cameraEntities, gridSize, canvasSize.width, canvasSize.height]);

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

      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      // Convert screen to world coordinates
      const worldX =
        (x - canvas.width / 2) / camera.zoom + camera.x;
      const worldY =
        (y - canvas.height / 2) / camera.zoom + camera.y;

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
          const rect = canvasRef.current?.getBoundingClientRect();
          if (!rect) return;
          const x = e.clientX - rect.left;
          const y = e.clientY - rect.top;
          const worldX = (x - canvas.width / 2) / camera.zoom + camera.x;
          const worldY = (y - canvas.height / 2) / camera.zoom + camera.y;
          onContextMenuOpen(
            { x: e.clientX, y: e.clientY },
            { x: worldX, y: worldY }
          );
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
