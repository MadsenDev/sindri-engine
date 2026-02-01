import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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

interface PlayViewProps {
  isPlaying: boolean;
}

export default function PlayView({ isPlaying }: PlayViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [camera, setCamera] = useState({
    x: 0,
    y: 0,
    zoom: 1,
    rotation: 0,
  });

  useEffect(() => {
    let active = true;
    const fetchCamera = async () => {
      try {
        const cameras = await invoke<CameraInfo[]>("camera_entities");
        if (!active) return;
        const activeCam = cameras.find((cam) => cam.active) || cameras[0];
        if (!activeCam) return;
        setCamera({
          x: activeCam.world_position[0] + activeCam.offset[0],
          y: activeCam.world_position[1] + activeCam.offset[1],
          zoom: activeCam.zoom,
          rotation: activeCam.rotation,
        });
      } catch {
        // ignore
      }
    };

    fetchCamera();
    const interval = window.setInterval(fetchCamera, 250);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animationFrameId: number;
    let lastTime = 0;
    const targetFPS = 30;
    const frameInterval = 1000 / targetFPS;

    const renderFrame = async () => {
      const width = canvas.width;
      const height = canvas.height;
      if (width === 0 || height === 0) return;

      try {
        const frame = await invoke<ViewportFrame>("viewport_render", {
          width,
          height,
          cameraX: camera.x,
          cameraY: camera.y,
          zoom: camera.zoom,
          rotation: camera.rotation,
        });

        const image = new ImageData(
          new Uint8ClampedArray(frame.rgba),
          frame.width,
          frame.height
        );
        ctx.putImageData(image, 0, 0);
      } catch (error) {
        console.error("Play view render failed:", error);
      }
    };

    const loop = async (currentTime: number) => {
      if (currentTime - lastTime >= frameInterval) {
        lastTime = currentTime;
        if (isPlaying) {
          await renderFrame();
        }
      }
      animationFrameId = requestAnimationFrame(loop);
    };

    animationFrameId = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(animationFrameId);
  }, [camera, isPlaying]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const resize = () => {
      const displayWidth = canvas.clientWidth;
      const displayHeight = canvas.clientHeight;

      if (canvas.width !== displayWidth || canvas.height !== displayHeight) {
        canvas.width = displayWidth;
        canvas.height = displayHeight;
      }
    };

    resize();
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      resizeObserver.disconnect();
    };
  }, []);

  return (
    <div className="game-preview-surface">
      <canvas ref={canvasRef} className="w-full h-full" />
      {!isPlaying && <div className="game-overlay">Game view</div>}
    </div>
  );
}
