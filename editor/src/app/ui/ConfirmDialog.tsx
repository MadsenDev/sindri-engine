interface ConfirmDialogProps {
  isOpen: boolean;
  onSave: () => void;
  onDiscard: () => void;
  onCancel: () => void;
}

export default function ConfirmDialog({
  isOpen,
  onSave,
  onDiscard,
  onCancel,
}: ConfirmDialogProps) {
  if (!isOpen) {
    return null;
  }

  return (
    <div className="confirm-overlay" onClick={onCancel}>
      <div className="confirm-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="confirm-title">Unsaved changes</div>
        <div className="confirm-body">
          Save changes to the current scene before continuing?
        </div>
        <div className="confirm-actions">
          <button className="unity-button" onClick={onSave}>
            Save
          </button>
          <button className="unity-button muted" onClick={onDiscard}>
            Don&apos;t Save
          </button>
          <button className="unity-button danger" onClick={onCancel}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
