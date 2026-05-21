import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { createPortal } from "react-dom";

interface ProjectSettings {
  name: string;
  resolution_width: number;
  resolution_height: number;
  pixel_art_mode: boolean;
}

interface EditorPrefs {
  auto_save_interval_secs: number;
}

interface WorldSettings {
  gravity_x: number;
  gravity_y: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
  projectPath: string | null;
  engineReady: boolean;
  initialTab?: "project" | "editor";
}

export default function SettingsModal({ open, onClose, projectPath, engineReady, initialTab = "project" }: Props) {
  const [tab, setTab] = useState<"project" | "editor">(initialTab);
  const [proj, setProj] = useState<ProjectSettings>({
    name: "My Game",
    resolution_width: 1280,
    resolution_height: 720,
    pixel_art_mode: true,
  });
  const [world, setWorld] = useState<WorldSettings>({ gravity_x: 0, gravity_y: 980 });
  const [prefs, setPrefs] = useState<EditorPrefs>({ auto_save_interval_secs: 5 });
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!open || !projectPath) return;
    invoke<ProjectSettings>("get_project_settings", { projectPath })
      .then(s => setProj(s))
      .catch(() => {});
    invoke<EditorPrefs>("get_editor_prefs")
      .then(p => setPrefs(p))
      .catch(() => {});
    if (engineReady) {
      fetch("http://localhost:7878/scene/world")
        .then(r => r.json())
        .then((w: WorldSettings) => setWorld(w))
        .catch(() => {});
    }
  }, [open, projectPath, engineReady]);

  useEffect(() => {
    setTab(initialTab);
  }, [initialTab, open]);

  if (!open) return null;

  const handleSave = async () => {
    if (!projectPath) return;
    setSaving(true);
    try {
      await invoke("save_project_settings", { projectPath, settings: proj });
      await invoke("save_editor_prefs", { prefs });
      if (engineReady) {
        await fetch("http://localhost:7878/scene/world", {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ gravity_x: world.gravity_x, gravity_y: world.gravity_y }),
        });
      }
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("Failed to save settings:", e);
    } finally {
      setSaving(false);
    }
  };

  const labelStyle = {
    fontSize: "11px",
    fontFamily: "var(--font-mono)",
    color: "var(--ink-3)",
    marginBottom: "4px",
    display: "block" as const,
  };

  const inputStyle = {
    width: "100%",
    height: "28px",
    padding: "0 8px",
    background: "var(--paper-2)",
    border: "1px solid var(--rule)",
    color: "var(--ink)",
    fontFamily: "var(--font-mono)",
    fontSize: "12px",
    boxSizing: "border-box" as const,
  };

  const fieldStyle = {
    marginBottom: "16px",
  };

  return createPortal(
    <div style={{
      position: "fixed", inset: 0,
      background: "rgba(0,0,0,0.55)",
      display: "flex", alignItems: "center", justifyContent: "center",
      zIndex: 9000,
    }} onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={{
        width: "480px",
        background: "var(--paper)",
        border: "1px solid var(--rule-2)",
        display: "flex", flexDirection: "column",
        maxHeight: "80vh",
      }}>
        {/* Header */}
        <div style={{
          height: "44px", display: "flex", alignItems: "center",
          padding: "0 20px", gap: "24px",
          borderBottom: "1px solid var(--rule)",
          flexShrink: 0,
        }}>
          <span style={{ fontFamily: "var(--font-ui)", fontWeight: 600, fontSize: "14px", color: "var(--ink)", flex: 1 }}>
            Settings
          </span>
          <button onClick={onClose} style={{
            width: "24px", height: "24px", background: "none", border: "none",
            color: "var(--ink-3)", fontSize: "16px", cursor: "pointer",
            display: "flex", alignItems: "center", justifyContent: "center",
          }}>×</button>
        </div>

        {/* Tabs */}
        <div style={{
          display: "flex", borderBottom: "1px solid var(--rule)",
          padding: "0 20px", gap: "0", flexShrink: 0,
        }}>
          {(["project", "editor"] as const).map(t => (
            <button key={t} onClick={() => setTab(t)} style={{
              padding: "10px 16px",
              fontSize: "12px", fontFamily: "var(--font-ui)",
              color: tab === t ? "var(--ink)" : "var(--ink-3)",
              fontWeight: tab === t ? 500 : 400,
              borderBottom: tab === t ? "2px solid var(--amber)" : "2px solid transparent",
              marginBottom: "-1px",
              background: "none", border: "none",
              borderBottomStyle: "solid",
              cursor: "pointer", textTransform: "capitalize",
            }}
              // Override the merged border above
              className={undefined}
            >{t === "project" ? "Project" : "Editor"}</button>
          ))}
        </div>

        {/* Body */}
        <div style={{ padding: "24px 24px 16px", overflowY: "auto", flex: 1 }}>
          {tab === "project" && (
            <>
              <div style={fieldStyle}>
                <label style={labelStyle}>Project Name</label>
                <input
                  style={inputStyle}
                  value={proj.name}
                  onChange={e => setProj(p => ({ ...p, name: e.target.value }))}
                />
              </div>

              <div style={fieldStyle}>
                <label style={labelStyle}>Resolution</label>
                <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                  <input
                    type="number" min={1} max={7680} step={1}
                    style={{ ...inputStyle, width: "80px" }}
                    value={proj.resolution_width}
                    onChange={e => setProj(p => ({ ...p, resolution_width: Math.max(1, parseInt(e.target.value) || 1280) }))}
                  />
                  <span style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "12px" }}>×</span>
                  <input
                    type="number" min={1} max={4320} step={1}
                    style={{ ...inputStyle, width: "80px" }}
                    value={proj.resolution_height}
                    onChange={e => setProj(p => ({ ...p, resolution_height: Math.max(1, parseInt(e.target.value) || 720) }))}
                  />
                  <span style={{ fontSize: "11px", color: "var(--ink-4)", fontFamily: "var(--font-mono)" }}>px</span>
                </div>
              </div>

              <div style={{ ...fieldStyle, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <div>
                  <span style={{ fontFamily: "var(--font-ui)", fontSize: "13px", color: "var(--ink)" }}>Pixel Art Mode</span>
                  <span style={{ display: "block", fontSize: "11px", color: "var(--ink-4)", fontFamily: "var(--font-mono)", marginTop: "2px" }}>
                    Nearest-neighbor filtering · pixel-perfect camera
                  </span>
                </div>
                <button
                  onClick={() => setProj(p => ({ ...p, pixel_art_mode: !p.pixel_art_mode }))}
                  style={{
                    width: "36px", height: "20px",
                    background: proj.pixel_art_mode ? "var(--amber)" : "var(--paper-3)",
                    border: "1px solid var(--rule-2)",
                    borderRadius: "10px",
                    position: "relative", cursor: "pointer", flexShrink: 0,
                    transition: "background 0.15s",
                  }}
                >
                  <span style={{
                    position: "absolute", top: "2px",
                    left: proj.pixel_art_mode ? "17px" : "2px",
                    width: "14px", height: "14px",
                    background: "var(--ink)",
                    borderRadius: "50%",
                    transition: "left 0.15s",
                  }} />
                </button>
              </div>

              <div style={{ ...fieldStyle, marginTop: "8px" }}>
                <label style={labelStyle}>Gravity</label>
                <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                  <label style={{ ...labelStyle, marginBottom: 0, marginRight: "4px", flexShrink: 0 }}>x</label>
                  <input
                    type="number" step="any"
                    style={{ ...inputStyle, width: "100px" }}
                    value={world.gravity_x}
                    onChange={e => setWorld(w => ({ ...w, gravity_x: parseFloat(e.target.value) || 0 }))}
                  />
                  <label style={{ ...labelStyle, marginBottom: 0, marginRight: "4px", flexShrink: 0 }}>y</label>
                  <input
                    type="number" step="any"
                    style={{ ...inputStyle, width: "100px" }}
                    value={world.gravity_y}
                    onChange={e => setWorld(w => ({ ...w, gravity_y: parseFloat(e.target.value) || 0 }))}
                  />
                </div>
                <span style={{ fontSize: "10px", color: "var(--ink-4)", fontFamily: "var(--font-mono)", marginTop: "4px", display: "block" }}>
                  Applies to the active scene · units/s²
                </span>
              </div>

              <div style={{
                padding: "12px", background: "var(--paper-2)",
                border: "1px solid var(--rule)",
                fontSize: "11px", color: "var(--ink-4)", fontFamily: "var(--font-mono)",
                lineHeight: "1.5",
              }}>
                Resolution and pixel art mode take effect after restarting the engine.
              </div>
            </>
          )}

          {tab === "editor" && (
            <>
              <div style={fieldStyle}>
                <label style={labelStyle}>Auto-save interval</label>
                <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                  <input
                    type="number" min={1} max={300} step={1}
                    style={{ ...inputStyle, width: "80px" }}
                    value={prefs.auto_save_interval_secs}
                    onChange={e => setPrefs(p => ({ ...p, auto_save_interval_secs: Math.max(1, parseInt(e.target.value) || 5) }))}
                  />
                  <span style={{ fontSize: "12px", color: "var(--ink-4)", fontFamily: "var(--font-mono)" }}>seconds</span>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div style={{
          height: "52px", borderTop: "1px solid var(--rule)",
          display: "flex", alignItems: "center", justifyContent: "flex-end",
          padding: "0 20px", gap: "10px", flexShrink: 0,
        }}>
          {saved && (
            <span style={{ fontSize: "11px", color: "var(--moss)", fontFamily: "var(--font-mono)" }}>Saved</span>
          )}
          <button onClick={onClose} style={{
            height: "28px", padding: "0 16px",
            background: "none", border: "1px solid var(--rule)",
            color: "var(--ink-3)", fontSize: "12px", fontFamily: "var(--font-ui)",
            cursor: "pointer",
          }}>Cancel</button>
          <button onClick={handleSave} disabled={saving || !projectPath} style={{
            height: "28px", padding: "0 16px",
            background: "var(--amber)", border: "none",
            color: "var(--paper)", fontSize: "12px", fontFamily: "var(--font-ui)",
            fontWeight: 600, cursor: "pointer", opacity: saving ? 0.6 : 1,
          }}>{saving ? "Saving…" : "Save"}</button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
