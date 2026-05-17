import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import Hierarchy from "./panels/Hierarchy";
import Inspector from "./panels/Inspector";
import AIChat from "./panels/AIChat";
import Viewport from "./panels/Viewport";
import ScriptEditor from "./panels/ScriptEditor";
import FileBrowser from "./panels/FileBrowser";
import WelcomeScreen from "./screens/WelcomeScreen";
import { ContextMenuProvider } from "./components/ContextMenu";

export interface Entity {
  id: number;
  name: string;
  parent: number | null;
  children: number[];
  active: boolean;
  components: Component[];
}

export type Component =
  | { type: "Transform"; x: number; y: number; scale_x: number; scale_y: number; rotation: number }
  | { type: "Sprite"; texture_path: string; width: number; height: number; flip_x: boolean; flip_y: boolean; color: [number, number, number, number] }
  | { type: "PhysicsBody"; body_type: "Dynamic" | "Kinematic" | "Fixed"; lock_rotation: boolean; linear_damping: number; angular_damping: number; collision_layer: number; collision_mask: number }
  | { type: "Collider"; width: number; height: number; offset_x: number; offset_y: number; is_trigger: boolean }
  | { type: "Script"; path: string }
  | { type: "Camera"; zoom: number; follow_entity: number | null }
  | { type: "AudioSource"; path: string; volume: number; looping: boolean; play_on_start: boolean };

export interface Scene {
  name: string;
  entities: Record<string, Entity>;
  next_id: number;
}

export interface AiAction {
  type: "edit_transform" | "write_script" | "attach_script" | "create_entity" | "delete_entity" | "rename_entity" | "add_component" | "remove_component" | "patch_component" | "suggest_fix";
  [key: string]: unknown;
}

export type ActiveTool = "select" | "move" | "scale" | "rotate";

export default function App() {
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [projectName, setProjectName] = useState<string>("");
  const [scene, setScene] = useState<Scene | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [selectedComponent, setSelectedComponent] = useState<number | null>(null); // index in entity.components
  const [openScript, setOpenScript] = useState<{ path: string; content: string } | null>(null);
  const [activeTool, setActiveTool] = useState<ActiveTool>("select");
  const [engineReady, setEngineReady] = useState(false);
  const [ollamaReady, setOllamaReady] = useState(false);
  const [ollamaModels, setOllamaModels] = useState<string[]>([]);
  const [selectedModel, setSelectedModel] = useState<string | null>(
    () => localStorage.getItem("sindri_selected_model")
  );
  const [screenshotB64, setScreenshotB64] = useState<string | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [leftTab, setLeftTab] = useState<"scene" | "files">("scene");
  const [projectFiles, setProjectFiles] = useState<{ path: string; kind: string; name: string }[]>([]);
  const screenshotInterval = useRef<ReturnType<typeof setInterval> | null>(null);

  const refreshScene = useCallback(async () => {
    try {
      const raw = await invoke<string>("get_scene");
      setScene(JSON.parse(raw));
    } catch {
      // engine not yet ready
    }
  }, []);

  // Poll engine health then start viewport polling
  useEffect(() => {
    let healthInterval: ReturnType<typeof setInterval>;

    const checkHealth = async () => {
      try {
        await invoke("get_scene");
        setEngineReady(true);
        clearInterval(healthInterval);
        await refreshScene();

        // Start viewport polling at 15fps
        screenshotInterval.current = setInterval(async () => {
          try {
            const b64 = await invoke<string>("get_screenshot");
            if (b64) setScreenshotB64(b64);
          } catch {
            // ignore
          }
        }, 67);
      } catch {
        // still waiting
      }
    };

    healthInterval = setInterval(checkHealth, 1000);
    checkHealth();

    return () => {
      clearInterval(healthInterval);
      if (screenshotInterval.current) clearInterval(screenshotInterval.current);
    };
  }, [refreshScene]);

  // Persist selected model
  useEffect(() => {
    if (selectedModel) localStorage.setItem("sindri_selected_model", selectedModel);
  }, [selectedModel]);

  // Check Ollama availability and fetch model list
  useEffect(() => {
    invoke<string[]>("list_ollama_models")
      .then(models => {
        setOllamaReady(true);
        setOllamaModels(models);
        // Only pick a default if nothing is saved
        if (models.length > 0 && !localStorage.getItem("sindri_selected_model")) {
          const preferred = models.find(m => m.includes("vl")) ?? models[0];
          setSelectedModel(preferred);
        }
      })
      .catch(() => setOllamaReady(false));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const selectedEntity = selectedId !== null && scene ? scene.entities[String(selectedId)] ?? null : null;

  const handleSceneChange = useCallback(() => {
    refreshScene();
  }, [refreshScene]);

  const handleTogglePlay = useCallback(async () => {
    const next = !isPlaying;
    try {
      await invoke("set_engine_paused", { paused: !next });
      setIsPlaying(next);
    } catch (err) {
      console.error("Failed to set play state:", err);
    }
  }, [isPlaying]);

  const handleOpenScript = useCallback(async (path: string) => {
    try {
      const content = await invoke<string>("get_script", { path });
      setOpenScript({ path, content });
    } catch (err) {
      console.error("Failed to open script:", err);
    }
  }, []);

  // Delete key removes selected entity
  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      if (e.key === "Delete" && selectedId !== null && document.activeElement?.tagName !== "INPUT" && document.activeElement?.tagName !== "TEXTAREA") {
        try {
          await invoke("apply_action", { action: { type: "delete_entity", entity_id: selectedId } });
          setSelectedId(null);
          handleSceneChange();
        } catch (err) {
          console.error("Failed to delete entity:", err);
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedId, handleSceneChange]);

  if (!projectPath) {
    return (
      <ContextMenuProvider>
        <div style={{ width: "100%", height: "100%" }}>
          <WelcomeScreen onOpen={(dir, name) => { setProjectPath(dir); setProjectName(name); }} />
        </div>
      </ContextMenuProvider>
    );
  }

  return (
    <ContextMenuProvider>
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <Header
        activeTool={activeTool}
        setActiveTool={setActiveTool}
        ollamaReady={ollamaReady}
        ollamaModels={ollamaModels}
        selectedModel={selectedModel}
        onSelectModel={setSelectedModel}
        isPlaying={isPlaying}
        onTogglePlay={handleTogglePlay}
        engineReady={engineReady}
      />

      <div style={{ display: "flex", flex: 1, overflow: "hidden", borderTop: "1px solid var(--border)" }}>
        {/* Left panel */}
        <div style={{
          width: "var(--panel-w-left)",
          minWidth: "var(--panel-w-left)",
          background: "var(--bg-1)",
          borderRight: "1px solid var(--border)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}>
          {/* Tab bar */}
          <div style={{
            display: "flex", height: "28px", borderBottom: "1px solid var(--border)", flexShrink: 0,
          }}>
            {(["scene", "files"] as const).map(tab => (
              <button
                key={tab}
                onClick={() => setLeftTab(tab)}
                style={{
                  flex: 1, background: "none", border: "none",
                  borderBottom: `2px solid ${leftTab === tab ? "var(--accent)" : "transparent"}`,
                  color: leftTab === tab ? "var(--text-bright)" : "var(--text-muted)",
                  fontFamily: "var(--font-ui)", fontSize: "10px", fontWeight: 600,
                  letterSpacing: "0.08em", textTransform: "uppercase",
                  cursor: "pointer", padding: "0",
                  transition: "color 0.1s",
                }}
              >{tab}</button>
            ))}
          </div>

          {leftTab === "scene" ? (
            <Hierarchy
              scene={scene}
              selectedId={selectedId}
              selectedComponent={selectedComponent}
              onSelect={id => { setSelectedId(id); setSelectedComponent(null); }}
              onSelectComponent={(entityId, idx) => { setSelectedId(entityId); setSelectedComponent(idx); }}
              onSceneChange={handleSceneChange}
              onDeleteEntity={async (id: number) => {
                await invoke("apply_action", { action: { type: "delete_entity", entity_id: id } });
                if (selectedId === id) { setSelectedId(null); setSelectedComponent(null); }
                handleSceneChange();
              }}
            />
          ) : (
            <FileBrowser
              projectPath={projectPath}
              onOpenScript={handleOpenScript}
              onFilesChange={setProjectFiles}
            />
          )}
        </div>

        {/* Center column */}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", minWidth: 0 }}>
          <Viewport
            scene={scene}
            selectedId={selectedId}
            onSelect={setSelectedId}
            screenshotB64={screenshotB64}
            engineReady={engineReady}
            isPlaying={isPlaying}
          />
          <ScriptEditor
            openScript={openScript}
            onClose={() => setOpenScript(null)}
          />
        </div>

        {/* Right panel */}
        <div style={{
          width: "var(--panel-w-right)",
          minWidth: "var(--panel-w-right)",
          background: "var(--bg-1)",
          borderLeft: "1px solid var(--border)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}>
          <Inspector
            entity={selectedEntity}
            selectedComponent={selectedComponent}
            onSelectComponent={type => setSelectedComponent(type)}
            onSceneChange={handleSceneChange}
            onOpenScript={handleOpenScript}
          />
          <AIChat
            scene={scene}
            openScript={openScript}
            screenshotB64={screenshotB64}
            selectedModel={selectedModel}
            projectFiles={projectFiles}
            onSceneChange={handleSceneChange}
          />
        </div>
      </div>

      <Footer
        engineReady={engineReady}
        ollamaReady={ollamaReady}
        scene={scene}
        projectName={projectName}
      />
    </div>
    </ContextMenuProvider>
  );
}

// ─── Header ────────────────────────────────────────────────────────────────

interface HeaderProps {
  activeTool: ActiveTool;
  setActiveTool: (t: ActiveTool) => void;
  ollamaReady: boolean;
  ollamaModels: string[];
  selectedModel: string | null;
  onSelectModel: (model: string) => void;
  isPlaying: boolean;
  onTogglePlay: () => void;
  engineReady: boolean;
}

function Header({ activeTool, setActiveTool, ollamaReady, ollamaModels, selectedModel, onSelectModel, isPlaying, onTogglePlay, engineReady }: HeaderProps) {
  const [modelDropdownOpen, setModelDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!modelDropdownOpen) return;
    const handler = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setModelDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [modelDropdownOpen]);
  const tools: { key: ActiveTool; icon: string; label: string }[] = [
    { key: "select", icon: "↖", label: "Select" },
    { key: "move", icon: "✥", label: "Move" },
    { key: "scale", icon: "⤢", label: "Scale" },
    { key: "rotate", icon: "↻", label: "Rotate" },
  ];

  return (
    <div style={{
      height: "var(--header-h)",
      background: "var(--bg-1)",
      borderBottom: "1px solid var(--border)",
      display: "flex",
      alignItems: "center",
      padding: "0 12px",
      gap: "8px",
      flexShrink: 0,
    }}>
      {/* Logo */}
      <div style={{ display: "flex", alignItems: "center", gap: "7px", marginRight: "12px" }}>
        {/* Isometric cube logo */}
        <svg viewBox="0 0 100 100" width="20" height="20" style={{ flexShrink: 0 }} xmlns="http://www.w3.org/2000/svg">
          <defs>
            <filter id="f2d-center-glow" x="-150%" y="-150%" width="400%" height="400%">
              <feGaussianBlur in="SourceGraphic" stdDeviation="3" result="b"/>
              <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
            </filter>
          </defs>
          {/* r=38 — dark blue-grey outer shell */}
          <polygon points="50,12 82.9,31 50,50 17.1,31" fill="#1d2233"/>
          <polygon points="82.9,31 82.9,69 50,88 50,50" fill="#131820"/>
          <polygon points="17.1,31 50,50 50,88 17.1,69" fill="#0e111a"/>
          {/* r=27 — transition to warm */}
          <polygon points="50,23 73.4,36.5 50,50 26.6,36.5" fill="#221808"/>
          <polygon points="73.4,36.5 73.4,63.5 50,77 50,50" fill="#181200"/>
          <polygon points="26.6,36.5 50,50 50,77 26.6,63.5" fill="#100d00"/>
          {/* r=18 — dark amber */}
          <polygon points="50,32 65.6,41 50,50 34.4,41" fill="#5a3200"/>
          <polygon points="65.6,41 65.6,59 50,68 50,50" fill="#482600"/>
          <polygon points="34.4,41 50,50 50,68 34.4,59" fill="#361b00"/>
          {/* r=11 — medium amber */}
          <polygon points="50,39 59.5,44.5 50,50 40.5,44.5" fill="#8c5000"/>
          <polygon points="59.5,44.5 59.5,55.5 50,61 50,50" fill="#763e00"/>
          <polygon points="40.5,44.5 50,50 50,61 40.5,55.5" fill="#5e3000"/>
          {/* r=6 — bright amber */}
          <polygon points="50,44 55.2,47 50,50 44.8,47" fill="#ca7400"/>
          <polygon points="55.2,47 55.2,53 50,56 50,50" fill="#aa5e00"/>
          <polygon points="44.8,47 50,50 50,56 44.8,53" fill="#8e4a00"/>
          {/* r=3 — golden */}
          <polygon points="50,47 52.6,48.5 50,50 47.4,48.5" fill="#eea030"/>
          <polygon points="52.6,48.5 52.6,51.5 50,53 50,50" fill="#d07e10"/>
          <polygon points="47.4,48.5 50,50 50,53 47.4,51.5" fill="#b86608"/>
          {/* Center white-hot glow */}
          <circle cx="50" cy="50" r="2.8" fill="#fff8e0" filter="url(#f2d-center-glow)"/>
          {/* Outer hex border */}
          <polygon points="50,12 82.9,31 82.9,69 50,88 17.1,69 17.1,31"
            fill="none" stroke="#7a4a00" strokeWidth="0.9"/>
          {/* Interior cube edges */}
          <line x1="50" y1="50" x2="82.9" y2="31" stroke="#2a3040" strokeWidth="0.5"/>
          <line x1="50" y1="50" x2="17.1" y2="31" stroke="#2a3040" strokeWidth="0.5"/>
          <line x1="50" y1="50" x2="50" y2="88" stroke="#1e2530" strokeWidth="0.5"/>
        </svg>
        <span style={{
          fontFamily: "var(--font-ui)",
          fontWeight: 800,
          fontSize: "14px",
          color: "var(--accent)",
          letterSpacing: "-0.02em",
        }}>SINDRI</span>
      </div>

      {/* Menu bar */}
      {["File", "Edit", "Scene", "View", "Build"].map(item => (
        <button key={item} style={{
          background: "none",
          border: "none",
          color: "var(--text-muted)",
          fontFamily: "var(--font-mono)",
          fontSize: "11px",
          padding: "3px 6px",
          cursor: "pointer",
          borderRadius: "var(--radius)",
        }}
          onMouseEnter={e => {
            (e.target as HTMLElement).style.background = "var(--bg-3)";
            (e.target as HTMLElement).style.color = "var(--text-bright)";
          }}
          onMouseLeave={e => {
            (e.target as HTMLElement).style.background = "none";
            (e.target as HTMLElement).style.color = "var(--text-muted)";
          }}
        >{item}</button>
      ))}

      {/* Separator */}
      <div style={{ width: "1px", height: "18px", background: "var(--border)" }} />

      {/* Tool buttons */}
      {tools.map(t => (
        <button key={t.key} title={t.label} onClick={() => setActiveTool(t.key)} style={{
          width: "28px",
          height: "26px",
          background: activeTool === t.key ? "var(--accent-glow)" : "none",
          border: "none",
          borderRadius: "var(--radius)",
          color: activeTool === t.key ? "var(--accent)" : "var(--text-muted)",
          fontSize: "13px",
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}>{t.icon}</button>
      ))}

      {/* Separator */}
      <div style={{ width: "1px", height: "18px", background: "var(--border)" }} />

      {/* Play / Pause button */}
      <button
        onClick={engineReady ? onTogglePlay : undefined}
        title={!engineReady ? "Waiting for engine…" : isPlaying ? "Pause" : "Play"}
        style={{
          background: !engineReady
            ? "var(--bg-3)"
            : isPlaying
              ? "rgba(230,100,60,0.15)"
              : "var(--accent)",
          border: isPlaying ? "1px solid rgba(230,100,60,0.5)" : "none",
          borderRadius: "var(--radius)",
          color: !engineReady ? "var(--text-muted)" : isPlaying ? "rgb(230,120,80)" : "var(--bg-0)",
          fontFamily: "var(--font-ui)",
          fontWeight: 700,
          fontSize: "11px",
          padding: "4px 12px",
          cursor: engineReady ? "pointer" : "default",
          display: "flex",
          alignItems: "center",
          gap: "6px",
          letterSpacing: "0.05em",
          minWidth: "70px",
          justifyContent: "center",
        }}
      >
        {isPlaying ? (
          <>
            <span style={{ display: "flex", gap: "2px", alignItems: "center" }}>
              <span style={{ width: "3px", height: "10px", background: "currentColor", borderRadius: "1px", display: "inline-block" }} />
              <span style={{ width: "3px", height: "10px", background: "currentColor", borderRadius: "1px", display: "inline-block" }} />
            </span>
            PAUSE
          </>
        ) : (
          <>
            <span style={{
              width: 0, height: 0,
              borderTop: "5px solid transparent",
              borderBottom: "5px solid transparent",
              borderLeft: `8px solid ${engineReady ? "var(--bg-0)" : "var(--text-muted)"}`,
              display: "inline-block",
            }} />
            PLAY
          </>
        )}
      </button>

      <div style={{ flex: 1 }} />

      {/* AI model selector */}
      <div ref={dropdownRef} style={{ position: "relative" }}>
        <button
          onClick={() => ollamaReady && setModelDropdownOpen(o => !o)}
          style={{
            display: "flex",
            alignItems: "center",
            gap: "6px",
            background: modelDropdownOpen ? "var(--bg-3)" : "var(--ai-glow)",
            border: "1px solid var(--ai-dim)",
            borderRadius: "var(--radius)",
            padding: "3px 8px",
            fontSize: "11px",
            color: "var(--ai)",
            cursor: ollamaReady ? "pointer" : "default",
          }}
        >
          <span style={{
            width: "6px",
            height: "6px",
            borderRadius: "50%",
            background: ollamaReady ? "var(--ai)" : "var(--text-muted)",
            animation: ollamaReady ? "pulse 2s infinite" : "none",
            display: "inline-block",
            flexShrink: 0,
          }} />
          <span style={{ maxWidth: "120px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {selectedModel ?? (ollamaReady ? "no model" : "ollama offline")}
          </span>
          {ollamaReady && <span style={{ fontSize: "9px", opacity: 0.6 }}>▾</span>}
        </button>

        {modelDropdownOpen && (
          <div style={{
            position: "absolute",
            top: "calc(100% + 4px)",
            right: 0,
            background: "var(--bg-2)",
            border: "1px solid var(--border)",
            borderRadius: "var(--radius)",
            boxShadow: "0 4px 16px rgba(0,0,0,0.4)",
            zIndex: 100,
            minWidth: "180px",
            overflow: "hidden",
          }}>
            <div style={{ padding: "4px 8px 2px", fontSize: "9px", color: "var(--text-dim)", letterSpacing: "0.1em", textTransform: "uppercase" }}>
              Installed models
            </div>
            {ollamaModels.length === 0 ? (
              <div style={{ padding: "6px 10px", fontSize: "11px", color: "var(--text-muted)" }}>No models found</div>
            ) : (
              ollamaModels.map(m => (
                <button key={m} onClick={() => { onSelectModel(m); setModelDropdownOpen(false); }} style={{
                  display: "block",
                  width: "100%",
                  textAlign: "left",
                  background: m === selectedModel ? "var(--ai-glow)" : "none",
                  border: "none",
                  borderLeft: `2px solid ${m === selectedModel ? "var(--ai)" : "transparent"}`,
                  color: m === selectedModel ? "var(--ai)" : "var(--text-bright)",
                  fontFamily: "var(--font-mono)",
                  fontSize: "11px",
                  padding: "6px 10px",
                  cursor: "pointer",
                }}
                  onMouseEnter={e => { if (m !== selectedModel) (e.currentTarget as HTMLElement).style.background = "var(--bg-3)"; }}
                  onMouseLeave={e => { if (m !== selectedModel) (e.currentTarget as HTMLElement).style.background = "none"; }}
                >{m}</button>
              ))
            )}
          </div>
        )}
      </div>

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }
      `}</style>
    </div>
  );
}

// ─── Footer ────────────────────────────────────────────────────────────────

interface FooterProps {
  engineReady: boolean;
  ollamaReady: boolean;
  scene: Scene | null;
  projectName: string;
}

function Footer({ engineReady, ollamaReady, scene, projectName }: FooterProps) {
  const entityCount = scene ? Object.keys(scene.entities).length : 0;

  return (
    <div style={{
      height: "var(--footer-h)",
      background: "var(--bg-0)",
      borderTop: "1px solid var(--border)",
      display: "flex",
      alignItems: "center",
      padding: "0 10px",
      gap: "14px",
      flexShrink: 0,
      fontSize: "11px",
      color: "var(--text-muted)",
    }}>
      <StatusDot color={engineReady ? "var(--green)" : "var(--text-muted)"} label="engine ready" />
      <StatusDot color={ollamaReady ? "var(--ai)" : "var(--text-muted)"} label="ollama · localhost:11434" />
      <StatusDot color="var(--accent)" label={projectName || (scene ? scene.name : "no project")} />

      <div style={{ flex: 1 }} />

      <span style={{ color: "var(--text-dim)" }}>{entityCount} entities</span>
      <span style={{ color: "var(--text-dim)" }}>sindri v0.1</span>
    </div>
  );
}

function StatusDot({ color, label }: { color: string; label: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "5px" }}>
      <span style={{ width: "6px", height: "6px", borderRadius: "50%", background: color, display: "inline-block" }} />
      <span>{label}</span>
    </div>
  );
}
