import { useEffect, useState } from "react";
import Splitter from "./Splitter";

interface SplitterOverlayProps {
  gridRef: React.RefObject<HTMLDivElement>;
  sceneRef: React.RefObject<HTMLDivElement>;
  hierarchyRef: React.RefObject<HTMLDivElement>;
  projectRef: React.RefObject<HTMLDivElement>;
  inspectorRef: React.RefObject<HTMLDivElement>;
  onResizeHorizontal1: (delta: number) => void;
  onResizeHorizontal2: (delta: number) => void;
  onResizeHorizontal3: (delta: number) => void;
}

export default function SplitterOverlay({
  gridRef,
  sceneRef,
  hierarchyRef,
  projectRef,
  inspectorRef,
  onResizeHorizontal1,
  onResizeHorizontal2,
  onResizeHorizontal3,
}: SplitterOverlayProps) {
  const [positions, setPositions] = useState<{
    horizontal1: { left: number; top: number; width: number; height: number } | null;
    horizontal2: { left: number; top: number; width: number; height: number } | null;
    horizontal3: { left: number; top: number; width: number; height: number } | null;
  }>({
    horizontal1: null,
    horizontal2: null,
    horizontal3: null,
  });

  useEffect(() => {
    const updatePositions = () => {
      if (!gridRef.current || !sceneRef.current || !hierarchyRef.current || !projectRef.current || !inspectorRef.current) {
        return;
      }

      const gridRect = gridRef.current.getBoundingClientRect();
      const sceneRect = sceneRef.current.getBoundingClientRect();
      const hierarchyRect = hierarchyRef.current.getBoundingClientRect();
      const projectRect = projectRef.current.getBoundingClientRect();

      // Horizontal splitter 1: between Scene/Game and Hierarchy
      const horizontal1Left = sceneRect.right - gridRect.left;
      const horizontal1Top = 0;
      const horizontal1Width = 8;
      const horizontal1Height = gridRect.height;

      // Horizontal splitter 2: between Hierarchy and Project
      const horizontal2Left = hierarchyRect.right - gridRect.left;
      const horizontal2Top = 0;
      const horizontal2Width = 8;
      const horizontal2Height = gridRect.height;

      // Horizontal splitter 3: between Project and Inspector
      const horizontal3Left = projectRect.right - gridRect.left;
      const horizontal3Top = 0;
      const horizontal3Width = 8;
      const horizontal3Height = gridRect.height;

      setPositions({
        horizontal1: {
          left: horizontal1Left - 4,
          top: horizontal1Top,
          width: horizontal1Width,
          height: horizontal1Height,
        },
        horizontal2: {
          left: horizontal2Left - 4,
          top: horizontal2Top,
          width: horizontal2Width,
          height: horizontal2Height,
        },
        horizontal3: {
          left: horizontal3Left - 4,
          top: horizontal3Top,
          width: horizontal3Width,
          height: horizontal3Height,
        },
      });
    };

    updatePositions();
    window.addEventListener("resize", updatePositions);
    const interval = setInterval(updatePositions, 100); // Update periodically

    return () => {
      window.removeEventListener("resize", updatePositions);
      clearInterval(interval);
    };
  }, [gridRef, sceneRef, hierarchyRef, projectRef, inspectorRef]);

  if (!positions.horizontal1 || !positions.horizontal2 || !positions.horizontal3) {
    return null;
  }

  return (
    <>
      {/* Horizontal splitter 1: Scene/Game <-> Hierarchy */}
      <div
        style={{
          position: "absolute",
          left: `${positions.horizontal1.left}px`,
          top: `${positions.horizontal1.top}px`,
          width: `${positions.horizontal1.width}px`,
          height: `${positions.horizontal1.height}px`,
          zIndex: 1000,
        }}
      >
        <Splitter direction="horizontal" onResize={onResizeHorizontal1} />
      </div>

      {/* Horizontal splitter 2: Hierarchy <-> Project */}
      <div
        style={{
          position: "absolute",
          left: `${positions.horizontal2.left}px`,
          top: `${positions.horizontal2.top}px`,
          width: `${positions.horizontal2.width}px`,
          height: `${positions.horizontal2.height}px`,
          zIndex: 1000,
        }}
      >
        <Splitter direction="horizontal" onResize={onResizeHorizontal2} />
      </div>

      {/* Horizontal splitter 3: Project <-> Inspector */}
      <div
        style={{
          position: "absolute",
          left: `${positions.horizontal3.left}px`,
          top: `${positions.horizontal3.top}px`,
          width: `${positions.horizontal3.width}px`,
          height: `${positions.horizontal3.height}px`,
          zIndex: 1000,
        }}
      >
        <Splitter direction="horizontal" onResize={onResizeHorizontal3} />
      </div>
    </>
  );
}
