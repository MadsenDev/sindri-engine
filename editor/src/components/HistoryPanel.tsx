export default function HistoryPanel({ undoLabels, redoLabels, onUndo, onRedo }: {
  undoLabels: string[];
  redoLabels: string[];
  onUndo: () => void;
  onRedo: () => void;
}) {
  const isEmpty = undoLabels.length === 0 && redoLabels.length === 0;
  return (
    <div style={{ flex: 1, overflow: "auto", padding: "8px 0" }}>
      {isEmpty ? (
        <div style={{ padding: "14px 16px", color: "var(--ink-3)", fontSize: "12.5px", fontFamily: "var(--font-ui)" }}>
          History will appear here as you work.
        </div>
      ) : (
        <>
          {[...redoLabels].reverse().map((label, i) => (
            <button key={`redo-${i}`} onClick={onRedo} style={{
              display: "flex", alignItems: "center", gap: "8px",
              width: "100%", padding: "6px 16px",
              background: "none", border: "none", cursor: "pointer",
              fontFamily: "var(--font-ui)", fontSize: "12px",
              color: "var(--ink-4)", textAlign: "left",
            }}
              onMouseEnter={e => (e.currentTarget as HTMLElement).style.background = "var(--paper-3)"}
              onMouseLeave={e => (e.currentTarget as HTMLElement).style.background = "none"}
            >
              <span style={{ width: "10px", height: "10px", border: "1px solid var(--rule-2)", flexShrink: 0, opacity: 0.4 }} />
              <span>{label}</span>
            </button>
          ))}

          <div style={{
            display: "flex", alignItems: "center", gap: "8px",
            padding: "5px 16px",
            borderTop: "1px solid var(--rule)", borderBottom: "1px solid var(--rule)",
          }}>
            <span style={{ width: "10px", height: "10px", background: "var(--amber)", flexShrink: 0 }} />
            <span style={{ fontSize: "11px", color: "var(--amber)", fontFamily: "var(--font-mono)" }}>current</span>
          </div>

          {[...undoLabels].reverse().map((label, i) => (
            <button key={`undo-${i}`} onClick={onUndo} style={{
              display: "flex", alignItems: "center", gap: "8px",
              width: "100%", padding: "6px 16px",
              background: "none", border: "none", cursor: "pointer",
              fontFamily: "var(--font-ui)", fontSize: "12px",
              color: "var(--ink-2)", textAlign: "left",
            }}
              onMouseEnter={e => (e.currentTarget as HTMLElement).style.background = "var(--paper-3)"}
              onMouseLeave={e => (e.currentTarget as HTMLElement).style.background = "none"}
            >
              <span style={{ width: "10px", height: "10px", background: "var(--rule-2)", flexShrink: 0 }} />
              <span>{label}</span>
            </button>
          ))}
        </>
      )}
    </div>
  );
}
