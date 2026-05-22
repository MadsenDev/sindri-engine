import type { Scene } from "../App";

interface StatusbarProps {
  engineReady: boolean;
  ollamaReady: boolean;
  scene: Scene | null;
  selectedModel: string | null;
  runtimeErrors: string[];
  suggestionModelPulling: boolean;
  suggestionModel: string | null;
}

const SUGGESTION_MODEL_LABEL = "qwen2.5:0.5b";

export default function Statusbar({ engineReady, ollamaReady, scene, selectedModel, runtimeErrors, suggestionModelPulling, suggestionModel }: StatusbarProps) {
  const entityCount = scene ? Object.keys(scene.entities).length : 0;
  const errorCount = runtimeErrors.length;

  return (
    <footer style={{
      height: "var(--footer-h)",
      background: "var(--paper-2)",
      borderTop: "1px solid var(--rule)",
      display: "flex",
      alignItems: "center",
      padding: "0 18px",
      gap: "18px",
      flexShrink: 0,
      fontFamily: "var(--font-mono)",
      fontSize: "11px",
      color: "var(--ink-4)",
    }}>
      <StatusPill dot={engineReady ? "var(--moss)" : "var(--ink-4)"} label="engine ready" />
      <StatusPill dot={ollamaReady ? "var(--moss)" : "var(--ink-4)"} label="ollama · localhost:11434" />
      {selectedModel && (
        <>
          <span style={{ color: "var(--ink-4)" }}>·</span>
          <span style={{ color: "var(--ink-4)" }}>{selectedModel}</span>
        </>
      )}
      {suggestionModelPulling && (
        <>
          <span style={{ color: "var(--ink-4)" }}>·</span>
          <span style={{ color: "var(--amber)", fontStyle: "italic" }}>pulling {SUGGESTION_MODEL_LABEL}…</span>
        </>
      )}
      {!suggestionModelPulling && suggestionModel && (
        <>
          <span style={{ color: "var(--ink-4)" }}>·</span>
          <span style={{ color: "var(--ink-4)" }}>hints · {suggestionModel}</span>
        </>
      )}
      <div style={{ flex: 1 }} />
      <span>{entityCount} entities</span>
      <span style={{ color: errorCount > 0 ? "var(--red)" : "var(--ink-4)" }}>
        {errorCount} errors
      </span>
      <span>sindri v0.1</span>
    </footer>
  );
}

function StatusPill({ dot, label }: { dot: string; label: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
      <span style={{ width: "6px", height: "6px", background: dot, display: "inline-block" }} />
      <span>{label}</span>
    </div>
  );
}
