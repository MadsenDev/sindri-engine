import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { decodeRawFrame } from "../app/frameDecoder";
import { presentImageData } from "../app/framePresenter";
import type { CameraInfo, PlayState } from "../app/types";

interface PlayViewProps {
  playState: PlayState;
  aspectRatio: null | [number, number];
  renderTick: number;
  camera: CameraInfo | null;
}

export default function PlayView({
  playState,
  aspectRatio,
  renderTick,
  camera,
}: PlayViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (playState === "stopped" || renderTick === 0 || !camera) {
      return;
    }
    const canvas = canvasRef.current;
    if (!canvas) return;

    const renderFrame = async () => {
      const width = canvas.width;
      const height = canvas.height;
      if (width === 0 || height === 0) return;

      try {
        const frame = await invoke<Uint8Array>("viewport_render_raw", {
          width,
          height,
          cameraX: camera.world_position[0] + camera.offset[0],
          cameraY: camera.world_position[1] + camera.offset[1],
          zoom: camera.zoom,
          rotation: camera.rotation,
        });

        const image = decodeRawFrame(frame, width, height);
        await presentImageData(canvas, image);
      } catch (error) {
        console.error("Play view render failed:", error);
      }
    };

    renderFrame();
  }, [camera, playState, renderTick]);

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

  const frameStyle = aspectRatio
    ? { aspectRatio: `${aspectRatio[0]} / ${aspectRatio[1]}` }
    : undefined;

  return (
    <div className="game-preview-surface">
      <div className="w-full h-full flex items-center justify-center">
        <canvas ref={canvasRef} className="w-full h-full" style={frameStyle} />
      </div>
      {playState === "stopped" && <div className="game-overlay">Game view</div>}
      {playState === "paused" && <div className="game-overlay">Paused</div>}
    </div>
  );
}
