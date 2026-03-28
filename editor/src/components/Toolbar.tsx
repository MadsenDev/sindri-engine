export type Tool = "move" | "rotate" | "scale";

import type { PlayState } from "../app/types";

interface ToolbarProps {
  currentTool: Tool;
  onToolChange: (tool: Tool) => void;
  playState: PlayState;
  onUndo?: () => void;
  onRedo?: () => void;
  canUndo?: boolean;
  canRedo?: boolean;
  onNewScene?: () => void;
  onSave?: () => void;
  onLoad?: () => void;
  onPlay?: () => void;
  onPause?: () => void;
  onResume?: () => void;
  onStep?: () => void;
  onStop?: () => void;
}

export default function Toolbar({
  currentTool,
  onToolChange,
  playState,
  onUndo = () => {},
  onRedo = () => {},
  canUndo = false,
  canRedo = false,
  onNewScene = () => {},
  onSave = () => {},
  onLoad = () => {},
  onPlay = () => {},
  onPause = () => {},
  onResume = () => {},
  onStep = () => {},
  onStop = () => {},
}: ToolbarProps) {
  const isPlaying = playState !== "stopped";

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
          <>
            {playState === "playing" ? (
              <button onClick={onPause} className="toolbar-button subtle">
                Ⅱ Pause
              </button>
            ) : (
              <button onClick={onResume} className="toolbar-button subtle">
                ▶ Resume
              </button>
            )}
            <button onClick={onStep} className="toolbar-button subtle" title="Step one frame">
              ⇥ Step
            </button>
            <button onClick={onStop} className="toolbar-button danger">
              ⏹ Stop
            </button>
          </>
        )}
      </div>
    </div>
  );
}
