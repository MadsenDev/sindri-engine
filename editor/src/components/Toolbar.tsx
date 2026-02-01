export type Tool = "move" | "rotate" | "scale";

interface ToolbarProps {
  currentTool: Tool;
  onToolChange: (tool: Tool) => void;
  isPlaying: boolean;
  onUndo?: () => void;
  onRedo?: () => void;
  canUndo?: boolean;
  canRedo?: boolean;
  onNewScene?: () => void;
  onSave?: () => void;
  onLoad?: () => void;
  onPlay?: () => void;
  onStop?: () => void;
}

export default function Toolbar({
  currentTool,
  onToolChange,
  isPlaying,
  onUndo = () => {},
  onRedo = () => {},
  canUndo = false,
  canRedo = false,
  onNewScene = () => {},
  onSave = () => {},
  onLoad = () => {},
  onPlay = () => {},
  onStop = () => {},
}: ToolbarProps) {
  return (
    <div className="toolbar">
      <div className="toolbar-group">
        <span className="toolbar-label">Tools</span>
        <button
          onClick={() => onToolChange("move")}
          disabled={isPlaying}
          className={`toolbar-button ${currentTool === "move" ? "active" : ""}`}
          title="Move Tool (W)"
        >
          ⤢ Move
        </button>
        <button
          onClick={() => onToolChange("rotate")}
          disabled={isPlaying}
          className={`toolbar-button ${currentTool === "rotate" ? "active" : ""}`}
          title="Rotate Tool (E)"
        >
          ↻ Rotate
        </button>
        <button
          onClick={() => onToolChange("scale")}
          disabled={isPlaying}
          className={`toolbar-button ${currentTool === "scale" ? "active" : ""}`}
          title="Scale Tool (R)"
        >
          ⤡ Scale
        </button>
      </div>

      <div className="toolbar-divider" />

      <div className="toolbar-group">
        <span className="toolbar-label">History</span>
        <button
          onClick={onUndo}
          disabled={!canUndo || isPlaying}
          className="toolbar-button subtle"
          title="Undo (Ctrl+Z)"
        >
          ↶ Undo
        </button>
        <button
          onClick={onRedo}
          disabled={!canRedo || isPlaying}
          className="toolbar-button subtle"
          title="Redo (Ctrl+Y)"
        >
          ↷ Redo
        </button>
      </div>

      <div className="toolbar-divider" />

      <div className="toolbar-group">
        <span className="toolbar-label">Scene</span>
        <button onClick={onNewScene} disabled={isPlaying} className="toolbar-button subtle">
          New
        </button>
        <button onClick={onSave} disabled={isPlaying} className="toolbar-button primary">
          Save
        </button>
        <button onClick={onLoad} disabled={isPlaying} className="toolbar-button subtle">
          Load
        </button>
      </div>

      <div className="flex-1" />

      <div className="toolbar-group">
        <span className="toolbar-label">Play</span>
        {!isPlaying ? (
          <button onClick={onPlay} className="toolbar-button success">
            ▶ Play
          </button>
        ) : (
          <button onClick={onStop} className="toolbar-button danger">
            ⏹ Stop
          </button>
        )}
      </div>
    </div>
  );
}

