import PlayView from "../components/PlayView";

interface PlayPanelProps {
  isPlaying: boolean;
}

export default function PlayPanel({ isPlaying }: PlayPanelProps) {
  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          <span className="panel-footnote muted">
            {isPlaying ? "Running" : "Stopped"}
          </span>
        </div>
      </div>
      <div className="panel-body muted-bg">
        <PlayView isPlaying={isPlaying} />
      </div>
    </div>
  );
}
