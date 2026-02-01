import { useRef, useEffect, useState } from "react";
import "./Splitter.css";

interface SplitterProps {
  direction: "horizontal" | "vertical";
  onResize: (delta: number) => void;
}

export default function Splitter({ direction, onResize }: SplitterProps) {
  const [isDragging, setIsDragging] = useState(false);
  const startPosRef = useRef(0);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const delta = direction === "horizontal" 
        ? e.clientX - startPosRef.current
        : e.clientY - startPosRef.current;
      onResize(delta);
      startPosRef.current = direction === "horizontal" ? e.clientX : e.clientY;
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDragging, direction, onResize]);

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    startPosRef.current = direction === "horizontal" ? e.clientX : e.clientY;
    setIsDragging(true);
  };

  return (
    <div
      className={`splitter splitter-${direction} ${isDragging ? "dragging" : ""}`}
      onMouseDown={handleMouseDown}
    />
  );
}

