import { useRef, useState } from "react";
import Editor, { type OnMount } from "@monaco-editor/react";
import type { Monaco } from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  openScript: { path: string; content: string } | null;
  onClose: () => void;
}

function beforeMount(monaco: Monaco) {
  monaco.editor.defineTheme("sindri-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "keyword", foreground: "c792ea" },
      { token: "type", foreground: "ffcb6b" },
      { token: "string", foreground: "c3e88d" },
      { token: "number", foreground: "e8a838" },
      { token: "comment", foreground: "3d4554" },
      { token: "function", foreground: "82aaff" },
    ],
    colors: {
      "editor.background": "#0a0b0d",
      "editor.foreground": "#c4cdd8",
      "editorLineNumber.foreground": "#3d4554",
      "editorLineNumber.activeForeground": "#5a6478",
      "editor.selectionBackground": "#e8a83825",
      "editor.lineHighlightBackground": "#0f1114",
      "editorCursor.foreground": "#e8a838",
      "editorGutter.background": "#0a0b0d",
    },
  });
}

export default function ScriptEditor({ openScript, onClose }: Props) {
  const editorRef = useRef<Parameters<OnMount>[0] | null>(null);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    if (!openScript || !editorRef.current) return;
    const content = editorRef.current.getValue();
    setSaving(true);
    try {
      await invoke("write_script", { path: openScript.path, content });
      setDirty(false);
    } catch (err) {
      console.error("Failed to save script:", err);
    } finally {
      setSaving(false);
    }
  };

  const handleMount: OnMount = (ed, monaco) => {
    editorRef.current = ed;
    // Ctrl+S / Cmd+S to save
    ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, save);
  };

  const fileName = openScript?.path.split("/").pop() ?? "";

  return (
    <div style={{
      height: "var(--script-h)",
      background: "var(--bg-0)",
      borderTop: "1px solid var(--border)",
      display: "flex",
      flexDirection: "column",
      flexShrink: 0,
    }}>
      {/* Tabs bar */}
      <div style={{
        height: "30px", background: "var(--bg-2)", borderBottom: "1px solid var(--border)",
        display: "flex", alignItems: "center", padding: "0 8px", gap: "2px", flexShrink: 0,
      }}>
        {openScript ? (
          <div style={{
            display: "flex", alignItems: "center", gap: "6px",
            padding: "4px 8px",
            background: "var(--bg-1)", border: "1px solid var(--border)",
            borderBottom: "1px solid var(--bg-1)",
            borderRadius: "var(--radius) var(--radius) 0 0",
            fontSize: "11px", color: "var(--text-bright)",
          }}>
            <span>{fileName}{dirty ? " ●" : ""}</span>
            <button onClick={onClose} style={{
              background: "none", border: "none", color: "var(--text-muted)",
              cursor: "pointer", fontSize: "12px", lineHeight: 1, padding: 0,
            }}>×</button>
          </div>
        ) : (
          <span style={{ fontSize: "11px", color: "var(--text-dim)", padding: "4px 8px" }}>
            no script open
          </span>
        )}

        {openScript && (
          <>
            <div style={{ flex: 1 }} />
            <button
              onClick={save}
              disabled={!dirty || saving}
              title="Save (Ctrl+S)"
              style={{
                background: dirty ? "var(--accent-glow)" : "none",
                border: `1px solid ${dirty ? "var(--accent-dim)" : "var(--border)"}`,
                borderRadius: "var(--radius)",
                color: dirty ? "var(--accent)" : "var(--text-dim)",
                fontFamily: "var(--font-mono)", fontSize: "10px",
                padding: "2px 8px", cursor: dirty ? "pointer" : "default",
              }}
            >{saving ? "saving…" : "save"}</button>
          </>
        )}
      </div>

      {/* Monaco editor */}
      <div style={{ flex: 1, overflow: "hidden" }}>
        <Editor
          height="100%"
          language="lua"
          value={openScript?.content ?? "-- Open a script from the Inspector to edit it"}
          theme="sindri-dark"
          beforeMount={beforeMount}
          onMount={handleMount}
          onChange={() => { if (openScript) setDirty(true); }}
          options={{
            fontSize: 12,
            fontFamily: "Geist Mono, monospace",
            lineNumbers: "on",
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            readOnly: !openScript,
            lineNumbersMinChars: 3,
            glyphMargin: true,
            folding: false,
            padding: { top: 6, bottom: 6 },
          }}
        />
      </div>
    </div>
  );
}
