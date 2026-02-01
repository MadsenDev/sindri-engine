import FileExplorer from "../components/FileExplorer";

interface ProjectPanelProps {
  refreshToken: number;
  onRefresh: () => void;
  onImportTexture: () => void;
}

export default function ProjectPanel({
  refreshToken,
  onRefresh,
  onImportTexture,
}: ProjectPanelProps) {
  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          <button className="unity-button muted" onClick={onImportTexture}>
            Import Texture
          </button>
          <button className="unity-button muted" onClick={onRefresh}>
            Refresh
          </button>
        </div>
      </div>
      <div className="panel-body muted-bg">
        <FileExplorer refreshToken={refreshToken} />
      </div>
    </div>
  );
}
