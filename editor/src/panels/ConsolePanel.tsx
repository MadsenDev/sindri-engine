interface ConsolePanelProps {
  statusMessage: string | null;
}

export default function ConsolePanel({ statusMessage }: ConsolePanelProps) {
  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          <span className="panel-footnote muted">Logs</span>
        </div>
      </div>
      <div className="panel-body console-body">
        <div className="console-line">Console is not wired yet.</div>
        {statusMessage && <div className="console-line">{statusMessage}</div>}
      </div>
    </div>
  );
}
