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
      { token: "keyword", foreground: "f0c050" },   // amber — AI/accent
      { token: "type",    foreground: "6dbcdb" },   // cyan
      { token: "string",  foreground: "9bb070" },   // moss
      { token: "number",  foreground: "6dbcdb" },   // cyan
      { token: "comment", foreground: "5a554e" },   // ink-4
      { token: "function",foreground: "c4beae" },   // ink-2
    ],
    colors: {
      "editor.background":              "#0d1117",
      "editor.foreground":              "#e6e1d4",
      "editorLineNumber.foreground":    "#5a554e",
      "editorLineNumber.activeForeground": "#8a8580",
      "editor.selectionBackground":     "#f0c05020",
      "editor.lineHighlightBackground": "#161b22",
      "editorCursor.foreground":        "#f0c050",
      "editorGutter.background":        "#0d1117",
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
      background: "var(--paper)",
      borderTop: "1px solid var(--rule)",
      display: "flex",
      flexDirection: "column",
      flexShrink: 0,
    }}>
      {/* Tabs bar */}
      <div style={{
        height: "36px", background: "var(--paper-2)",
        borderBottom: "1px solid var(--rule)",
        display: "flex", alignItems: "center",
        paddingRight: "14px", flexShrink: 0,
      }}>
        {openScript ? (
          <div style={{
            display: "flex", alignItems: "center", gap: "8px",
            padding: "0 16px", height: "100%",
            fontFamily: "var(--font-mono)", fontSize: "11.5px",
            color: "var(--ink)",
            background: "var(--paper)",
            borderRight: "1px solid var(--rule)",
          }}>
            <span style={{ fontSize: "11px", color: "var(--ink-3)" }}>⚡</span>
            <span>{fileName}</span>
            {dirty && <span style={{ color: "var(--amber)", marginLeft: "2px" }}>●</span>}
            <button onClick={onClose} style={{
              background: "none", border: "none", color: "var(--ink-4)",
              cursor: "pointer", fontSize: "13px", lineHeight: 1, padding: 0,
              marginLeft: "4px",
            }}>×</button>
          </div>
        ) : (
          <span style={{
            fontFamily: "var(--font-mono)", fontSize: "11px",
            color: "var(--ink-4)", padding: "0 16px",
          }}>
            ✦ open a script, or ask the assistant to write one
          </span>
        )}

        {openScript && (
          <>
            <div style={{ flex: 1 }} />
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)", marginRight: "12px" }}>
              lua
            </span>
            <button
              onClick={save}
              disabled={!dirty || saving}
              title="Save (Ctrl+S)"
              style={{
                background: "none",
                border: `1px solid ${dirty ? "var(--rule-2)" : "transparent"}`,
                color: dirty ? "var(--ink-2)" : "var(--ink-4)",
                fontFamily: "var(--font-mono)", fontSize: "11px",
                padding: "3px 10px", cursor: dirty ? "pointer" : "default",
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
            fontSize: 12.5,
            fontFamily: "'JetBrains Mono', 'VT323', monospace",
            lineNumbers: "on",
            lineHeight: 1.6,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            readOnly: !openScript,
            lineNumbersMinChars: 4,
            glyphMargin: false,
            folding: false,
            padding: { top: 8, bottom: 8 },
          }}
        />
      </div>
    </div>
  );
}
