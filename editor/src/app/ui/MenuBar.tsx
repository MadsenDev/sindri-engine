import type { MouseEvent as ReactMouseEvent } from "react";
import type { PanelDefinition } from "../types";

interface MenuBarProps {
  hasProject: boolean;
  projectName: string | null;
  sceneDirty: boolean;
  statusMessage: string | null;
  panelMenuOpen: boolean;
  panelDefinitions: PanelDefinition[];
  onTogglePanelMenu: () => void;
  onOpenPanel: (panelId: string) => void;
  onResetLayout: () => void;
  onNewScene: () => void;
  onSave: () => void;
  onSaveAs: () => void;
  onLoad: () => void;
  onCloseProject: () => void;
  onMinimize: () => void;
  onToggleMaximize: () => void;
  onCloseWindow: () => void;
  onStartDragging: (event: ReactMouseEvent) => void;
}

export default function MenuBar({
  hasProject,
  projectName,
  sceneDirty,
  statusMessage,
  panelMenuOpen,
  panelDefinitions,
  onTogglePanelMenu,
  onOpenPanel,
  onResetLayout,
  onNewScene,
  onSave,
  onSaveAs,
  onLoad,
  onCloseProject,
  onMinimize,
  onToggleMaximize,
  onCloseWindow,
  onStartDragging,
}: MenuBarProps) {
  return (
    <div className="unity-menu-bar">
      <div className="menu-left">
        <div
          className="menu-logo"
          data-tauri-drag-region
          onMouseDown={onStartDragging}
          onDoubleClick={onToggleMaximize}
        >
          Forge2D
        </div>
        {hasProject && (
          <div className="menu-items">
            <button className="menu-item" onClick={onNewScene}>
              New
            </button>
            <button className="menu-item" onClick={onSave}>
              Save
            </button>
            <button className="menu-item" onClick={onSaveAs}>
              Save As
            </button>
            <button className="menu-item" onClick={onLoad}>
              Load
            </button>
            <div className="create-menu">
              <button
                className="menu-item"
                onClick={(event) => {
                  event.stopPropagation();
                  onTogglePanelMenu();
                }}
              >
                Panels
              </button>
              {panelMenuOpen && (
                <div className="create-menu-list">
                  {panelDefinitions.map((panel) => (
                    <button
                      key={panel.id}
                      className="create-menu-item"
                      onClick={() => onOpenPanel(panel.id)}
                    >
                      {panel.name}
                    </button>
                  ))}
                  <button className="create-menu-item" onClick={onResetLayout}>
                    Reset Layout
                  </button>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
      <div
        className="menu-drag-region"
        data-tauri-drag-region
        onMouseDown={onStartDragging}
        onDoubleClick={onToggleMaximize}
      />
      <div className="menu-right">
        {hasProject && (
          <>
            <div
              className="menu-status"
              data-tauri-drag-region
              onMouseDown={onStartDragging}
              onDoubleClick={onToggleMaximize}
            >
              {projectName}
              {sceneDirty ? " *" : ""}
            </div>
            <button className="menu-item" onClick={onCloseProject}>
              Close Project
            </button>
            {statusMessage && <div className="menu-status">{statusMessage}</div>}
          </>
        )}
        <div className="window-controls">
          <button
            className="window-control"
            onClick={onMinimize}
            aria-label="Minimize"
            title="Minimize"
          >
            _
          </button>
          <button
            className="window-control"
            onClick={onToggleMaximize}
            aria-label="Maximize"
            title="Maximize"
          >
            [ ]
          </button>
          <button
            className="window-control close"
            onClick={onCloseWindow}
            aria-label="Close"
            title="Close"
          >
            X
          </button>
        </div>
      </div>
    </div>
  );
}
