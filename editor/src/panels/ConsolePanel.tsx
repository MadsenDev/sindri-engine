import type { ConsoleEntry } from "../app/types";

interface ConsolePanelProps {
  statusMessage: string | null;
  entries: ConsoleEntry[];
}

export default function ConsolePanel({ statusMessage, entries }: ConsolePanelProps) {
  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          <span className="panel-footnote muted">Logs</span>
        </div>
      </div>
      <div className="panel-body console-body">
        {statusMessage && <div className="console-line">{statusMessage}</div>}
        {entries.length === 0 ? (
          <div className="console-line">No log output yet.</div>
        ) : (
          entries.map((entry) => (
            <div key={entry.id} className="console-line">
              [{entry.timestamp}] {entry.level.toUpperCase()} {entry.message}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
