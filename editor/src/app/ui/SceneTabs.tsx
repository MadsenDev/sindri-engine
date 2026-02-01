import type { SceneTab } from "../types";

interface SceneTabsProps {
  tabs: SceneTab[];
  activeTabId: string | null;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onAdd: () => void;
}

export default function SceneTabs({
  tabs,
  activeTabId,
  onSelect,
  onClose,
  onAdd,
}: SceneTabsProps) {
  return (
    <div className="scene-tabs">
      <div className="scene-tabs-list">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            className={`scene-tab ${tab.id === activeTabId ? "active" : ""}`}
            onClick={() => onSelect(tab.id)}
          >
            <span className="scene-tab-title">
              {tab.name}
              {tab.isDirty ? " *" : ""}
            </span>
            <span
              className="scene-tab-close"
              onClick={(event) => {
                event.stopPropagation();
                onClose(tab.id);
              }}
              role="button"
              aria-label={`Close ${tab.name}`}
            >
              ×
            </span>
          </button>
        ))}
      </div>
      <button className="scene-tab-add" onClick={onAdd}>
        +
      </button>
    </div>
  );
}
