import Inspector from "../components/Inspector";

interface InspectorPanelProps {
  selectedEntityId: number | null;
  selectedEntityName: string | null;
  inspectorRefresh: number;
}

export default function InspectorPanel({
  selectedEntityId,
  selectedEntityName,
  inspectorRefresh,
}: InspectorPanelProps) {
  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          {selectedEntityId !== null && (
            <span className="panel-footnote">
              {selectedEntityName ?? `Entity ${selectedEntityId}`}
            </span>
          )}
        </div>
      </div>
      <div className="panel-body muted-bg" style={{ overflow: "hidden", padding: 0 }}>
        <Inspector selectedEntityId={selectedEntityId} refreshTrigger={inspectorRefresh} />
      </div>
    </div>
  );
}
