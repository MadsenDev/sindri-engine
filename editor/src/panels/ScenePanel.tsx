import type { RefObject } from "react";
import Viewport, { type ViewportHandle } from "../components/Viewport";
import type { EntityInfo } from "../app/types";
import type { Tool } from "../components/Toolbar";

interface PresetOption {
  id: string;
  label: string;
  preset: string;
}

interface ScenePanelProps {
  createMenuOpen: boolean;
  onToggleCreateMenu: () => void;
  presetOptions: PresetOption[];
  onCreateEntity: (preset: string, world?: { x: number; y: number }) => void;
  onFrameSelection: () => void;
  onResetCamera: () => void;
  selectedEntityIds: number[];
  selectedEntityId: number | null;
  gridSize: number;
  onGridSizeChange: (value: number) => void;
  viewportRef: RefObject<ViewportHandle>;
  entities: EntityInfo[];
  onEntityClick: (entityId: number) => void;
  onSelectionChange: (ids: number[]) => void;
  onTransformChange: () => void | Promise<void>;
  onContextMenuOpen: (
    screen: { x: number; y: number },
    world?: { x: number; y: number }
  ) => void;
  onAssetDrop: (
    asset: { path: string; kind: string },
    world: { x: number; y: number }
  ) => void;
  isPlaying: boolean;
  tool: Tool;
}

export default function ScenePanel({
  createMenuOpen,
  onToggleCreateMenu,
  presetOptions,
  onCreateEntity,
  onFrameSelection,
  onResetCamera,
  selectedEntityIds,
  selectedEntityId,
  gridSize,
  onGridSizeChange,
  viewportRef,
  entities,
  onEntityClick,
  onSelectionChange,
  onTransformChange,
  onContextMenuOpen,
  onAssetDrop,
  isPlaying,
  tool,
}: ScenePanelProps) {
  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          <div className="create-menu">
            <button
              className="unity-button muted"
              onClick={(event) => {
                event.stopPropagation();
                onToggleCreateMenu();
              }}
            >
              + Create
            </button>
            {createMenuOpen && (
              <div className="create-menu-list">
                {presetOptions.map((option) => (
                  <button
                    key={option.id}
                    className="create-menu-item"
                    onClick={() => onCreateEntity(option.preset)}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            )}
          </div>
          <button className="unity-button muted" onClick={onFrameSelection}>
            Frame
          </button>
          <button className="unity-button muted" onClick={onResetCamera}>
            Reset
          </button>
        </div>
      </div>
      <div className="panel-body scene-body">
        <div className="viewport-toolbar">
          <span className="viewport-pill">
            Selection: {selectedEntityIds.length || (selectedEntityId ? 1 : 0)}
          </span>
          <label className="viewport-pill muted">
            Grid
            <input
              className="viewport-grid-input"
              type="number"
              min={5}
              value={gridSize}
              onChange={(e) => onGridSizeChange(Number(e.target.value) || 10)}
            />
          </label>
        </div>
        <div className="viewport-surface">
          <Viewport
            ref={viewportRef}
            entities={entities}
            selectedEntityId={selectedEntityId}
            selectedEntityIds={selectedEntityIds}
            onEntityClick={onEntityClick}
            onSelectionChange={onSelectionChange}
            onTransformChange={onTransformChange}
            onContextMenuOpen={onContextMenuOpen}
            onAssetDrop={onAssetDrop}
            isPlaying={isPlaying}
            tool={tool}
          />
          {isPlaying && <div className="mode-banner">PLAY</div>}
        </div>
      </div>
    </div>
  );
}
