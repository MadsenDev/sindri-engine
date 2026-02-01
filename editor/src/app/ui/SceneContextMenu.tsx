interface PresetOption {
  id: string;
  label: string;
  preset: string;
}

interface SceneContextMenuProps {
  x: number;
  y: number;
  world?: { x: number; y: number };
  presetOptions: PresetOption[];
  showSelectionActions: boolean;
  onCreateEntity: (preset: string, world?: { x: number; y: number }) => void;
  onDuplicate: () => void;
  onDelete: () => void;
}

export default function SceneContextMenu({
  x,
  y,
  world,
  presetOptions,
  showSelectionActions,
  onCreateEntity,
  onDuplicate,
  onDelete,
}: SceneContextMenuProps) {
  return (
    <div
      className="context-menu"
      style={{ left: x, top: y }}
      onClick={(e) => e.stopPropagation()}
    >
      <div className="context-menu-section">
        {presetOptions.map((option) => (
          <button
            key={option.id}
            className="context-menu-item"
            onClick={() => onCreateEntity(option.preset, world)}
          >
            Create {option.label}
          </button>
        ))}
      </div>
      {showSelectionActions && (
        <div className="context-menu-section">
          <button className="context-menu-item" onClick={onDuplicate}>
            Duplicate
          </button>
          <button className="context-menu-item" onClick={onDelete}>
            Delete
          </button>
        </div>
      )}
    </div>
  );
}
