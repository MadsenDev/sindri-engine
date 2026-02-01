import { useState, useRef, useEffect, ReactNode, forwardRef } from "react";
import Tab, { TabData } from "./Tab";
import "./ResizablePanel.css";

export interface PanelProps {
  id: string;
  tabs: TabData[];
  activeTabId: string | null;
  onTabActivate: (panelId: string, tabId: string) => void;
  onTabClose: (panelId: string, tabId: string) => void;
  onTabDragStart: (e: React.DragEvent, panelId: string, tabId: string) => void;
  onTabDrop: (e: React.DragEvent, panelId: string) => void;
  headerActions?: ReactNode;
  footer?: ReactNode;
  mutedBg?: boolean;
  className?: string;
  children?: ReactNode;
  minWidth?: number;
  minHeight?: number;
  defaultWidth?: number;
  defaultHeight?: number;
  resizable?: {
    horizontal?: boolean;
    vertical?: boolean;
  };
  onResize?: (width: number, height: number) => void;
}

const ResizablePanel = forwardRef<HTMLDivElement, PanelProps>(function ResizablePanel({
  id,
  tabs,
  activeTabId,
  onTabActivate,
  onTabClose,
  onTabDragStart,
  onTabDrop,
  headerActions,
  footer,
  mutedBg = false,
  className = "",
  children,
  minWidth = 100,
  minHeight = 100,
  defaultWidth,
  defaultHeight,
  resizable = { horizontal: true, vertical: true },
  onResize,
}, ref) {
  const [width, setWidth] = useState(defaultWidth);
  const [height, setHeight] = useState(defaultHeight);
  const [isResizing, setIsResizing] = useState(false);
  const [resizeDirection, setResizeDirection] = useState<"horizontal" | "vertical" | null>(null);
  const internalRef = useRef<HTMLDivElement>(null);
  const startPosRef = useRef({ x: 0, y: 0, width: 0, height: 0 });

  // Use forwarded ref or internal ref
  const panelRef = (ref as React.MutableRefObject<HTMLDivElement | null>) || internalRef;

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!panelRef.current || !resizeDirection) return;

      const deltaX = e.clientX - startPosRef.current.x;
      const deltaY = e.clientY - startPosRef.current.y;

      if (resizeDirection === "horizontal") {
        const newWidth = Math.max(minWidth, startPosRef.current.width + deltaX);
        setWidth(newWidth);
        if (onResize) onResize(newWidth, height || 0);
      } else if (resizeDirection === "vertical") {
        const newHeight = Math.max(minHeight, startPosRef.current.height + deltaY);
        setHeight(newHeight);
        if (onResize) onResize(width || 0, newHeight);
      }
    };

    const handleMouseUp = () => {
      setIsResizing(false);
      setResizeDirection(null);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing, resizeDirection, minWidth, minHeight, width, height, onResize]);

  const handleResizeStart = (direction: "horizontal" | "vertical") => (e: React.MouseEvent) => {
    e.preventDefault();
    if (!panelRef.current) return;

    const rect = panelRef.current.getBoundingClientRect();
    startPosRef.current = {
      x: e.clientX,
      y: e.clientY,
      width: rect.width,
      height: rect.height,
    };
    setIsResizing(true);
    setResizeDirection(direction);
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    onTabDrop(e, id);
  };

  const style: React.CSSProperties = {};
  if (width !== undefined) style.width = `${width}px`;
  if (height !== undefined) style.height = `${height}px`;

  return (
    <div
      ref={panelRef}
      className={`panel unity-panel ${className}`}
      style={style}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      <header className="panel-header tight">
        <div className="panel-tabs">
          {tabs.map((tab) => (
            <Tab
              key={tab.id}
              tab={tab}
              isActive={activeTabId === tab.id}
              onActivate={() => onTabActivate(id, tab.id)}
              onClose={() => onTabClose(id, tab.id)}
              onDragStart={(e, tabId) => onTabDragStart(e, id, tabId)}
            />
          ))}
        </div>
        {headerActions && <div className="panel-actions">{headerActions}</div>}
      </header>
      <div className={`panel-body ${mutedBg ? "muted-bg" : ""}`}>
        {children}
      </div>
      {footer && <footer className="panel-footer tight">{footer}</footer>}
      {resizable.horizontal && (
        <div
          className="resize-handle resize-handle-right"
          onMouseDown={handleResizeStart("horizontal")}
        />
      )}
      {resizable.vertical && (
        <div
          className="resize-handle resize-handle-bottom"
          onMouseDown={handleResizeStart("vertical")}
        />
      )}
    </div>
  );
});

export default ResizablePanel;
