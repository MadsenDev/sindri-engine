import "./Tab.css";

export interface TabData {
  id: string;
  label: string;
  content: React.ReactNode;
  closable?: boolean;
}

interface TabProps {
  tab: TabData;
  isActive: boolean;
  onActivate: () => void;
  onClose?: () => void;
  onDragStart?: (e: React.DragEvent, tabId: string) => void;
}

export default function Tab({ tab, isActive, onActivate, onClose, onDragStart }: TabProps) {

  const handleClose = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (onClose) {
      onClose();
    }
  };

  const handleDragStart = (e: React.DragEvent) => {
    if (onDragStart) {
      onDragStart(e, tab.id);
    }
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", tab.id);
  };

  return (
    <span
      className={`panel-tab ${isActive ? "active" : ""}`}
      onClick={onActivate}
      draggable
      onDragStart={handleDragStart}
    >
      {tab.label}
      {tab.closable !== false && (
        <button
          className="tab-close"
          onClick={handleClose}
          onMouseDown={(e) => e.stopPropagation()}
        >
          ×
        </button>
      )}
    </span>
  );
}

