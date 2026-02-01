import Inspector from "../components/Inspector";

interface InspectorPanelProps {
  selectedEntityId: number | null;
  inspectorRefresh: number;
}

export default function InspectorPanel({
  selectedEntityId,
  inspectorRefresh,
}: InspectorPanelProps) {
  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          {selectedEntityId !== null && (
            <span className="panel-footnote">Entity {selectedEntityId}</span>
          )}
        </div>
      </div>
      <div className="panel-body muted-bg">
        <Inspector selectedEntityId={selectedEntityId} refreshTrigger={inspectorRefresh} />
      </div>
    </div>
  );
}
