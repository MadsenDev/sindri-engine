import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

const RECENT_KEY = "sindri_recent_projects";
const MAX_RECENT = 8;

interface RecentProject {
  name: string;
  path: string;
  openedAt: number;
}

function getRecent(): RecentProject[] {
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
  } catch {
    return [];
  }
}

function addRecent(name: string, path: string) {
  const list = getRecent().filter(r => r.path !== path);
  list.unshift({ name, path, openedAt: Date.now() });
  localStorage.setItem(RECENT_KEY, JSON.stringify(list.slice(0, MAX_RECENT)));
}

interface Props {
  onOpen: (projectDir: string, projectName: string) => void;
}

type Mode = "home" | "new" | "open";

export default function WelcomeScreen({ onOpen }: Props) {
  const [mode, setMode] = useState<Mode>("home");
  const [recent, setRecent] = useState<RecentProject[]>([]);

  // New project form
  const [newName, setNewName] = useState("");
  const [newLocation, setNewLocation] = useState("");
  const [newError, setNewError] = useState("");
  const [newBusy, setNewBusy] = useState(false);

  // Open project form
  const [openPath, setOpenPath] = useState("");
  const [openError, setOpenError] = useState("");
  const [openBusy, setOpenBusy] = useState(false);

  const pickNewLocation = async () => {
    const dir = await open({ directory: true, multiple: false, title: "Choose project location" });
    if (dir) setNewLocation(dir as string);
  };

  const pickOpenPath = async () => {
    const dir = await open({ directory: true, multiple: false, title: "Open Sindri project" });
    if (dir) setOpenPath(dir as string);
  };

  useEffect(() => {
    setRecent(getRecent());
  }, []);

  const handleCreate = async () => {
    const name = newName.trim();
    const loc = newLocation.trim();
    if (!name) { setNewError("Project name is required"); return; }
    if (!loc) { setNewError("Location is required"); return; }
    setNewError("");
    setNewBusy(true);
    try {
      const projectDir = await invoke<string>("create_project", { parentDir: loc, name });
      await invoke("start_engine", { projectDir });
      addRecent(name, projectDir);
      onOpen(projectDir, name);
    } catch (err) {
      setNewError(String(err));
    } finally {
      setNewBusy(false);
    }
  };

  const handleOpen = async (dir?: string) => {
    const path = (dir ?? openPath).trim();
    if (!path) { setOpenError("Project path is required"); return; }
    setOpenError("");
    setOpenBusy(true);
    try {
      await invoke("start_engine", { projectDir: path });
      const name = path.split("/").pop() ?? path;
      addRecent(name, path);
      onOpen(path, name);
    } catch (err) {
      setOpenError(String(err));
    } finally {
      setOpenBusy(false);
    }
  };

  return (
    <div style={{
      width: "100%", height: "100%",
      background: "var(--bg-0)",
      display: "flex",
      flexDirection: "column",
      alignItems: "center",
      justifyContent: "center",
      position: "relative",
      overflow: "hidden",
    }}>
      {/* Background grid */}
      <div style={{
        position: "absolute", inset: 0, opacity: 0.03,
        backgroundImage: "linear-gradient(var(--border) 1px, transparent 1px), linear-gradient(90deg, var(--border) 1px, transparent 1px)",
        backgroundSize: "40px 40px",
        pointerEvents: "none",
      }} />

      <div style={{ position: "relative", width: "100%", maxWidth: "520px", padding: "0 24px" }}>
        {/* Logo */}
        <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "8px" }}>
          <div style={{
            width: "32px", height: "32px",
            background: "var(--accent)",
            clipPath: "polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)",
            flexShrink: 0,
          }} />
          <div>
            <div style={{
              fontFamily: "var(--font-ui)", fontWeight: 800, fontSize: "24px",
              color: "var(--accent)", letterSpacing: "-0.03em",
            }}>SINDRI</div>
            <div style={{ fontSize: "11px", color: "var(--text-dim)", letterSpacing: "0.08em" }}>
              2D GAME ENGINE
            </div>
          </div>
        </div>

        <div style={{ height: "1px", background: "var(--border)", margin: "24px 0" }} />

        {mode === "home" && (
          <>
            {/* Action cards */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "10px", marginBottom: "28px" }}>
              <ActionCard
                icon="✦"
                title="New Project"
                desc="Start from scratch"
                onClick={() => setMode("new")}
              />
              <ActionCard
                icon="↗"
                title="Open Project"
                desc="Open an existing project"
                onClick={() => setMode("open")}
              />
            </div>

            {/* Recent projects */}
            {recent.length > 0 && (
              <div>
                <div style={{
                  fontSize: "9px", fontFamily: "var(--font-ui)", fontWeight: 600,
                  color: "var(--text-dim)", letterSpacing: "0.12em", textTransform: "uppercase",
                  marginBottom: "8px",
                }}>Recent</div>
                <div style={{ display: "flex", flexDirection: "column", gap: "2px" }}>
                  {recent.map(r => (
                    <button key={r.path} onClick={() => handleOpen(r.path)} style={{
                      display: "flex", alignItems: "center", gap: "10px",
                      background: "none", border: "1px solid transparent",
                      borderRadius: "var(--radius)", padding: "8px 10px",
                      cursor: "pointer", textAlign: "left", width: "100%",
                    }}
                      onMouseEnter={e => {
                        (e.currentTarget as HTMLElement).style.background = "var(--bg-2)";
                        (e.currentTarget as HTMLElement).style.borderColor = "var(--border)";
                      }}
                      onMouseLeave={e => {
                        (e.currentTarget as HTMLElement).style.background = "none";
                        (e.currentTarget as HTMLElement).style.borderColor = "transparent";
                      }}
                    >
                      <span style={{ fontSize: "14px", flexShrink: 0 }}>📁</span>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ fontSize: "12px", color: "var(--text-bright)", fontFamily: "var(--font-ui)", fontWeight: 600 }}>
                          {r.name}
                        </div>
                        <div style={{
                          fontSize: "10px", color: "var(--text-dim)",
                          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                        }}>{r.path}</div>
                      </div>
                      <span style={{ fontSize: "10px", color: "var(--text-dim)", flexShrink: 0 }}>
                        {formatAge(r.openedAt)}
                      </span>
                    </button>
                  ))}
                </div>
                {openError && <div style={{ fontSize: "11px", color: "var(--red)", marginTop: "8px" }}>{openError}</div>}
              </div>
            )}
          </>
        )}

        {mode === "new" && (
          <div>
            <button onClick={() => setMode("home")} style={backBtnStyle}>← Back</button>
            <div style={{ fontSize: "14px", color: "var(--text-bright)", fontWeight: 600, marginBottom: "16px", fontFamily: "var(--font-ui)" }}>
              New Project
            </div>
            <FormField label="Name">
              <input
                value={newName}
                onChange={e => setNewName(e.target.value)}
                onKeyDown={e => e.key === "Enter" && handleCreate()}
                placeholder="my-game"
                autoFocus
                style={inputStyle}
              />
            </FormField>
            <FormField label="Location" hint="parent directory — project folder will be created inside">
              <button onClick={pickNewLocation} style={folderPickerStyle}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: newLocation ? "var(--text-bright)" : "var(--text-dim)", flex: 1, textAlign: "left", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {newLocation || "Choose location…"}
                </span>
                <span style={{ color: "var(--accent)", fontSize: "11px", flexShrink: 0 }}>Browse</span>
              </button>
            </FormField>
            {newName && newLocation && (
              <div style={{ fontSize: "10px", color: "var(--text-dim)", marginBottom: "12px" }}>
                Will create: <span style={{ color: "var(--text-base)", fontFamily: "var(--font-mono)" }}>
                  {newLocation.replace(/\/$/, "")}/{newName}
                </span>
              </div>
            )}
            {newError && <div style={{ fontSize: "11px", color: "var(--red)", marginBottom: "8px" }}>{newError}</div>}
            <button onClick={handleCreate} disabled={newBusy} style={primaryBtnStyle(newBusy)}>
              {newBusy ? "Creating…" : "Create Project"}
            </button>
          </div>
        )}

        {mode === "open" && (
          <div>
            <button onClick={() => setMode("home")} style={backBtnStyle}>← Back</button>
            <div style={{ fontSize: "14px", color: "var(--text-bright)", fontWeight: 600, marginBottom: "16px", fontFamily: "var(--font-ui)" }}>
              Open Project
            </div>
            <FormField label="Project directory">
              <button onClick={pickOpenPath} style={folderPickerStyle} autoFocus>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: openPath ? "var(--text-bright)" : "var(--text-dim)", flex: 1, textAlign: "left", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {openPath || "Choose project folder…"}
                </span>
                <span style={{ color: "var(--accent)", fontSize: "11px", flexShrink: 0 }}>Browse</span>
              </button>
            </FormField>
            {openError && <div style={{ fontSize: "11px", color: "var(--red)", marginBottom: "8px" }}>{openError}</div>}
            <button onClick={() => handleOpen()} disabled={openBusy} style={primaryBtnStyle(openBusy)}>
              {openBusy ? "Opening…" : "Open Project"}
            </button>
          </div>
        )}

        {/* Version tag */}
        <div style={{
          position: "absolute", bottom: "-48px", right: 0,
          fontSize: "10px", color: "var(--text-dim)",
        }}>sindri v0.1</div>
      </div>
    </div>
  );
}

function ActionCard({ icon, title, desc, onClick }: { icon: string; title: string; desc: string; onClick: () => void }) {
  const [hovered, setHovered] = useState(false);
  return (
    <button
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        background: hovered ? "var(--bg-2)" : "var(--bg-1)",
        border: `1px solid ${hovered ? "var(--accent-dim)" : "var(--border)"}`,
        borderRadius: "6px",
        padding: "16px",
        cursor: "pointer",
        textAlign: "left",
        transition: "border-color 0.1s",
      }}
    >
      <div style={{ fontSize: "20px", marginBottom: "8px", color: "var(--accent)" }}>{icon}</div>
      <div style={{ fontSize: "12px", color: "var(--text-bright)", fontFamily: "var(--font-ui)", fontWeight: 600, marginBottom: "3px" }}>
        {title}
      </div>
      <div style={{ fontSize: "10px", color: "var(--text-muted)" }}>{desc}</div>
    </button>
  );
}

function FormField({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: "12px" }}>
      <div style={{ fontSize: "10px", color: "var(--text-muted)", marginBottom: "4px", letterSpacing: "0.05em" }}>
        {label}
        {hint && <span style={{ color: "var(--text-dim)", marginLeft: "6px" }}>— {hint}</span>}
      </div>
      {children}
    </div>
  );
}

function formatAge(ts: number): string {
  const diff = Date.now() - ts;
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return mins <= 1 ? "just now" : `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

const inputStyle: React.CSSProperties = {
  width: "100%", boxSizing: "border-box",
  background: "var(--bg-2)", border: "1px solid var(--border)",
  borderRadius: "var(--radius)", color: "var(--text-bright)",
  fontFamily: "var(--font-mono)", fontSize: "12px",
  padding: "7px 10px", outline: "none",
};

const folderPickerStyle: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: "10px",
  width: "100%", boxSizing: "border-box",
  background: "var(--bg-2)", border: "1px solid var(--border)",
  borderRadius: "var(--radius)", padding: "7px 10px",
  cursor: "pointer", textAlign: "left",
};

const backBtnStyle: React.CSSProperties = {
  background: "none", border: "none", color: "var(--text-muted)",
  fontFamily: "var(--font-mono)", fontSize: "11px",
  padding: "0", cursor: "pointer", marginBottom: "16px",
  display: "block",
};

const primaryBtnStyle = (disabled: boolean): React.CSSProperties => ({
  background: disabled ? "var(--bg-3)" : "var(--accent)",
  border: "none", borderRadius: "var(--radius)",
  color: disabled ? "var(--text-muted)" : "var(--bg-0)",
  fontFamily: "var(--font-ui)", fontWeight: 700,
  fontSize: "12px", padding: "8px 20px",
  cursor: disabled ? "default" : "pointer",
  letterSpacing: "0.05em",
});
