import React, { useState, useEffect, useCallback, useRef, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import Hierarchy from "./panels/Hierarchy";
import Inspector from "./panels/Inspector";
import ProposalsLane from "./panels/ProposalsLane";

import Viewport, { type TransformChange, type ColliderChange } from "./panels/Viewport";
import ScriptEditor from "./panels/ScriptEditor";
import FileBrowser from "./panels/FileBrowser";
import WelcomeScreen from "./screens/WelcomeScreen";
import { ContextMenuProvider } from "./components/ContextMenu";
import CmdK from "./components/CmdK";
import SettingsModal from "./components/SettingsModal";

const SUGGESTION_MODEL_LABEL = "qwen2.5:0.5b";

export type AiModelRole = "assistant" | "code" | "fast" | "vision";
export type AiModelConfig = Record<AiModelRole, string | null>;

const AI_MODEL_ROLES: { role: AiModelRole; title: string; detail: string }[] = [
  { role: "assistant", title: "Assistant", detail: "General reasoning, scene planning, entity/component changes." },
  { role: "code", title: "Code", detail: "Lua scripts, engine API usage, structured code edits." },
  { role: "fast", title: "Fast", detail: "Small local model for hints, summaries, and cheap checks." },
  { role: "vision", title: "Vision", detail: "Viewport screenshots and visual debugging." },
];

function preferredOllamaModel(models: string[], role: AiModelRole = "assistant") {
  const lower = (needle: string) => models.find(m => m.toLowerCase().includes(needle));
  if (role === "code") return lower("coder") ?? lower("qwen3") ?? models[0] ?? null;
  if (role === "fast") return lower("1.5b") ?? lower("3b") ?? lower("4b") ?? models[0] ?? null;
  if (role === "vision") return lower("vl") ?? lower("gemma3") ?? lower("vision") ?? null;
  return lower("qwen3") ?? lower("mistral") ?? lower("gemma3:12b") ?? lower("coder") ?? models[0] ?? null;
}

function defaultModelConfig(models: string[]): AiModelConfig {
  return {
    assistant: preferredOllamaModel(models, "assistant"),
    code: preferredOllamaModel(models, "code"),
    fast: preferredOllamaModel(models, "fast"),
    vision: preferredOllamaModel(models, "vision"),
  };
}

function loadModelConfig(): AiModelConfig {
  try {
    const parsed = JSON.parse(localStorage.getItem("sindri_ai_model_roles") ?? "{}") as Partial<AiModelConfig>;
    return {
      assistant: parsed.assistant ?? localStorage.getItem("sindri_selected_model"),
      code: parsed.code ?? null,
      fast: parsed.fast ?? null,
      vision: parsed.vision ?? null,
    };
  } catch {
    return { assistant: localStorage.getItem("sindri_selected_model"), code: null, fast: null, vision: null };
  }
}

export interface Entity {
  id: number;
  name: string;
  parent: number | null;
  children: number[];
  active: boolean;
  staged: boolean;
  components: Component[];
}

export interface AnimClip {
  name: string;
  start_frame: number;
  end_frame: number;
  fps: number;
  looping: boolean;
}

export interface TilePalette {
  name: string;
  texture_path: string;
  tileset_cols: number;
  tileset_rows: number;
  margin: number;
  spacing: number;
  solid_tiles: number[];
}

export interface TileLayer {
  name: string;
  tiles: number[];
  visible: boolean;
  opacity: number;
}

export type Component =
  | { type: "Transform"; x: number; y: number; scale_x: number; scale_y: number; rotation: number }
  | { type: "Sprite"; texture_path: string; width: number; height: number; flip_x: boolean; flip_y: boolean; color: [number, number, number, number] }
  | { type: "AnimatedSprite"; texture_path: string; cols: number; rows: number; width: number; height: number; flip_x: boolean; flip_y: boolean; tint: [number, number, number, number]; clips: AnimClip[]; default_clip: string; margin?: number; spacing?: number }
  | { type: "PhysicsBody"; body_type: "Dynamic" | "Kinematic" | "Fixed"; lock_rotation: boolean; linear_damping: number; angular_damping: number; collision_layer: number; collision_mask: number }
  | { type: "Collider"; width: number; height: number; offset_x: number; offset_y: number; is_trigger: boolean }
  | { type: "Script"; path: string }
  | { type: "Camera"; active?: boolean; zoom: number; follow_entity: number | null; offset_x?: number; offset_y?: number; bounds_min_x?: number | null; bounds_min_y?: number | null; bounds_max_x?: number | null; bounds_max_y?: number | null; smoothing?: number; dead_zone_width?: number; dead_zone_height?: number; pixel_perfect?: boolean }
  | { type: "AudioSource"; path: string; volume: number; looping: boolean; play_on_start: boolean }
  | { type: "Tilemap"; palettes: TilePalette[]; tile_width: number; tile_height: number; map_cols: number; map_rows: number; layers: TileLayer[]; tint: [number, number, number, number] };

export interface Scene {
  name: string;
  entities: Record<string, Entity>;
  next_id: number;
}

export interface AiAction {
  type: "edit_transform" | "write_script" | "attach_script" | "create_entity" | "delete_entity" | "rename_entity" | "add_component" | "remove_component" | "patch_component" | "suggest_fix";
  [key: string]: unknown;
}

export interface ProposalChange {
  id: string;
  label: string;
  detail: string;
  staged_entity_ids: number[];
  modified_entity_ids: number[];
  new_script_paths: string[];
  script_backups: { path: string; existed: boolean; content: string }[];
}

export interface ProposalData {
  prompt: string;
  summary: string;
  changes: ProposalChange[];
}


export type ActiveTool = "select" | "move" | "scale" | "rotate" | "collider";
type PlaybackState = "stopped" | "playing" | "paused";
type TransformSnapshot = TransformChange["before"];
interface EngineStatus {
  status: string;
  model: string;
  paused: boolean;
  playback?: string | null;
  error_count?: number | null;
}

type AiProvider = "ollama" | "openai" | "anthropic";

interface AiProviderStatus {
  provider: AiProvider;
  configured: boolean;
  defaultModel: string;
}

export default function App() {
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [projectName, setProjectName] = useState<string>("");
  const [scene, setScene] = useState<Scene | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [selectedComponent, setSelectedComponent] = useState<number | null>(null);
  const [openScript, setOpenScript] = useState<{ path: string; content: string } | null>(null);
  const [activeTool, setActiveTool] = useState<ActiveTool>("select");
  const [engineReady, setEngineReady] = useState(false);
  const [ollamaReady, setOllamaReady] = useState(false);
  const [ollamaModels, setOllamaModels] = useState<string[]>([]);
  const [modelConfig, setModelConfig] = useState<AiModelConfig>(() => loadModelConfig());
  const [selectedProvider, setSelectedProvider] = useState<AiProvider>(
    () => (localStorage.getItem("sindri_ai_provider") as AiProvider | null) ?? "ollama"
  );
  const [providerStatuses, setProviderStatuses] = useState<AiProviderStatus[]>([]);
  const [aiSettingsOpen, setAiSettingsOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [projectSettings, setProjectSettings] = useState<{ name: string; resolution_width: number; resolution_height: number; pixel_art_mode: boolean } | null>(null);
  const [playbackState, setPlaybackState] = useState<PlaybackState>("stopped");
  const [leftTab, setLeftTab] = useState<"scene" | "files" | "history">("scene");
  const [projectFiles, setProjectFiles] = useState<{ path: string; kind: string; name: string }[]>([]);
  const [runtimeErrors, setRuntimeErrors] = useState<string[]>([]);
  const [sceneDirty, setSceneDirty] = useState(false);
  const [undoDepth, setUndoDepth] = useState(0);
  const [redoDepth, setRedoDepth] = useState(0);
  const [cmdKOpen, setCmdKOpen] = useState(false);
  const [cmdKInit, setCmdKInit] = useState<{ message?: string; input?: string } | null>(null);
  const [pendingProposal, setPendingProposal] = useState<ProposalData | null>(null);
  const [suggestionModel, setSuggestionModel] = useState<string | null>(null);
  const [suggestionModelPulling, setSuggestionModelPulling] = useState(false);
  const undoStack = useRef<(TransformChange | ColliderChange)[]>([]);
  const redoStack = useRef<(TransformChange | ColliderChange)[]>([]);
  const [undoLabels, setUndoLabels] = useState<string[]>([]);
  const [redoLabels, setRedoLabels] = useState<string[]>([]);
  const sceneRestoredRef = useRef(false);

  const refreshScene = useCallback(async () => {
    try {
      const raw = await invoke<string>("get_scene");
      setScene(JSON.parse(raw));
    } catch {
      // engine not yet ready
    }
  }, []);

  useEffect(() => {
    let healthInterval: ReturnType<typeof setInterval>;
    const checkHealth = async () => {
      try {
        const status = await invoke<EngineStatus>("get_engine_status");
        setEngineReady(true);
        const playback = status.playback;
        if (playback === "playing" || playback === "paused" || playback === "stopped") {
          setPlaybackState(playback);
        }
        clearInterval(healthInterval);
        await refreshScene();
        if (projectPath) {
          invoke<{ name: string; resolution_width: number; resolution_height: number; pixel_art_mode: boolean }>("get_project_settings", { projectPath })
            .then(s => setProjectSettings(s)).catch(() => {});
        }
        // Restore staged proposal from disk if any staged entities exist.
        try {
          const pending = await invoke<ProposalData | null>("get_pending_proposal");
          if (pending) setPendingProposal(pending);
        } catch { /* no pending proposal */ }
      } catch {
        // still waiting
      }
    };
    healthInterval = setInterval(checkHealth, 1000);
    checkHealth();
    return () => clearInterval(healthInterval);
  }, [refreshScene]);

  // Restore the last-opened scene for this project after engine first becomes ready.
  useEffect(() => {
    if (!engineReady || !projectPath || sceneRestoredRef.current) return;
    sceneRestoredRef.current = true;
    const lastScene = localStorage.getItem(`sindri_last_scene:${projectPath}`);
    if (!lastScene) return;
    invoke("open_scene_file", { projectPath, relativePath: lastScene })
      .then(() => refreshScene())
      .catch(() => { /* last scene may have been deleted — engine already has main.sindri */ });
  }, [engineReady, projectPath, refreshScene]);

  useEffect(() => {
    if (!engineReady) return;
    let cancelled = false;
    let failures = 0;
    const FAILURE_THRESHOLD = 4;
    let interval: ReturnType<typeof setInterval> | null = null;
    const sync = async () => {
      try {
        const status = await invoke<EngineStatus>("get_engine_status");
        if (cancelled) return;
        failures = 0;
        const playback = status.playback;
        if (playback === "playing" || playback === "paused" || playback === "stopped") {
          setPlaybackState(playback);
        }
      } catch {
        if (!cancelled) {
          failures++;
          if (failures >= FAILURE_THRESHOLD) {
            setEngineReady(false);
            setPlaybackState("stopped");
          }
        }
      }
    };
    interval = setInterval(sync, 1000);
    return () => { cancelled = true; if (interval) clearInterval(interval); };
  }, [engineReady]);

  useEffect(() => {
    if (!engineReady) return;
    let cancelled = false;
    let timeout: ReturnType<typeof setTimeout> | null = null;
    const tick = async () => {
      await refreshScene();
      if (!cancelled) timeout = setTimeout(tick, playbackState === "playing" ? 33 : 500);
    };
    tick();
    return () => { cancelled = true; if (timeout) clearTimeout(timeout); };
  }, [engineReady, playbackState, refreshScene]);

  useEffect(() => {
    if (!engineReady) return;
    let cancelled = false;
    let timeout: ReturnType<typeof setTimeout> | null = null;
    const tick = async () => {
      try {
        const errors = await invoke<string[]>("get_runtime_errors");
        if (!cancelled) setRuntimeErrors(errors);
      } catch {
        if (!cancelled) setRuntimeErrors([]);
      }
      if (!cancelled) timeout = setTimeout(tick, 750);
    };
    tick();
    return () => { cancelled = true; if (timeout) clearTimeout(timeout); };
  }, [engineReady]);

  useEffect(() => {
    localStorage.setItem("sindri_ai_model_roles", JSON.stringify(modelConfig));
    if (modelConfig.assistant) {
      localStorage.setItem("sindri_selected_model", modelConfig.assistant);
    } else {
      localStorage.removeItem("sindri_selected_model");
    }
  }, [modelConfig]);

  useEffect(() => {
    localStorage.setItem("sindri_ai_provider", selectedProvider);
  }, [selectedProvider]);

  const refreshProviderStatuses = useCallback(async () => {
    try {
      setProviderStatuses(await invoke<AiProviderStatus[]>("get_ai_provider_status"));
    } catch {
      setProviderStatuses([]);
    }
  }, []);

  useEffect(() => {
    invoke<string[]>("list_ollama_models")
      .then(models => {
        setOllamaReady(true);
        setOllamaModels(models);
        if (selectedProvider === "ollama" && models.length > 0) {
          setModelConfig(current => {
            const defaults = defaultModelConfig(models);
            const next: AiModelConfig = { ...current };
            for (const role of AI_MODEL_ROLES.map(item => item.role)) {
              if (!next[role] || !models.includes(next[role] ?? "")) {
                next[role] = defaults[role];
              }
            }
            return next;
          });
        }
      })
      .catch(() => setOllamaReady(false));
    refreshProviderStatuses();
  }, [refreshProviderStatuses, selectedProvider]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!ollamaReady || suggestionModel || suggestionModelPulling) return;
    const cached = localStorage.getItem("sindri_suggestion_model");
    if (cached) { setSuggestionModel(cached); return; }
    setSuggestionModelPulling(true);
    invoke<string>("ensure_suggestion_model")
      .then(model => {
        setSuggestionModel(model);
        localStorage.setItem("sindri_suggestion_model", model);
      })
      .catch(() => { /* will fall back to rules-based */ })
      .finally(() => setSuggestionModelPulling(false));
  }, [ollamaReady]); // eslint-disable-line react-hooks/exhaustive-deps

  const selectedEntity = selectedId !== null && scene ? scene.entities[String(selectedId)] ?? null : null;

  const syncHistoryDepths = useCallback(() => {
    setUndoDepth(undoStack.current.length);
    setRedoDepth(redoStack.current.length);
  }, []);

  const handleSceneChange = useCallback(() => {
    setSceneDirty(true);
    refreshScene();
  }, [refreshScene]);

  const patchTransformSnapshot = useCallback(async (entityId: number, transform: TransformSnapshot) => {
    await invoke("patch_transform", {
      entityId, x: transform.x, y: transform.y,
      scaleX: transform.scale_x, scaleY: transform.scale_y, rotation: transform.rotation,
    });
  }, []);

  const labelForChange = useCallback((change: TransformChange | ColliderChange, entityName?: string): string => {
    const name = entityName ?? `#${change.entityId}`;
    if ("scale_x" in change.before) {
      const tc = change as TransformChange;
      const ds = Math.abs(tc.after.scale_x - tc.before.scale_x) + Math.abs(tc.after.scale_y - tc.before.scale_y);
      const dr = Math.abs(tc.after.rotation - tc.before.rotation);
      if (ds > 0.001) return `Scale ${name}`;
      if (dr > 0.001) return `Rotate ${name}`;
      return `Move ${name}`;
    }
    return `Resize Collider ${name}`;
  }, []);

  const applyChange = useCallback(async (change: TransformChange | ColliderChange, direction: "undo" | "redo") => {
    const data = direction === "undo" ? change.before : change.after;
    if ("scale_x" in data) {
      await patchTransformSnapshot(change.entityId, data as TransformSnapshot);
    } else {
      await invoke("patch_component", {
        entityId: change.entityId,
        componentIdx: (change as ColliderChange).componentIdx,
        data,
      });
    }
  }, [patchTransformSnapshot]);

  const handleTransformCommit = useCallback(async (change: TransformChange) => {
    try {
      await patchTransformSnapshot(change.entityId, change.after);
      const label = labelForChange(change, scene?.entities[String(change.entityId)]?.name);
      undoStack.current.push(change);
      redoStack.current = [];
      setUndoLabels(prev => [...prev, label]);
      setRedoLabels([]);
      syncHistoryDepths();
      setSceneDirty(true);
      await refreshScene();
    } catch (err) {
      console.error("Failed to patch transform:", err);
    }
  }, [labelForChange, patchTransformSnapshot, refreshScene, scene, syncHistoryDepths]);

  const handleColliderCommit = useCallback(async (change: ColliderChange) => {
    try {
      await invoke("patch_component", {
        entityId: change.entityId,
        componentIdx: change.componentIdx,
        data: change.after,
      });
      const label = labelForChange(change, scene?.entities[String(change.entityId)]?.name);
      undoStack.current.push(change);
      redoStack.current = [];
      setUndoLabels(prev => [...prev, label]);
      setRedoLabels([]);
      syncHistoryDepths();
      setSceneDirty(true);
      await refreshScene();
    } catch (err) {
      console.error("Failed to patch collider:", err);
    }
  }, [labelForChange, refreshScene, scene, syncHistoryDepths]);

  const undoTransform = useCallback(async () => {
    const change = undoStack.current.pop();
    if (!change) return;
    try {
      await applyChange(change, "undo");
      redoStack.current.push(change);
      setUndoLabels(prev => { const n = [...prev]; const moved = n.pop()!; setRedoLabels(r => [moved, ...r]); return n; });
      syncHistoryDepths();
      setSceneDirty(true);
      await refreshScene();
    } catch (err) {
      undoStack.current.push(change);
      syncHistoryDepths();
      console.error("Failed to undo:", err);
    }
  }, [applyChange, refreshScene, syncHistoryDepths]);

  const redoTransform = useCallback(async () => {
    const change = redoStack.current.pop();
    if (!change) return;
    try {
      await applyChange(change, "redo");
      undoStack.current.push(change);
      setRedoLabels(prev => { const n = [...prev]; const moved = n.shift()!; setUndoLabels(u => [...u, moved]); return n; });
      syncHistoryDepths();
      setSceneDirty(true);
      await refreshScene();
    } catch (err) {
      redoStack.current.push(change);
      syncHistoryDepths();
      console.error("Failed to redo:", err);
    }
  }, [applyChange, refreshScene, syncHistoryDepths]);

  const saveScene = useCallback(async () => {
    try {
      await invoke("save_scene");
      setSceneDirty(false);
    } catch (err) {
      console.error("Failed to save scene:", err);
    }
  }, []);

  const handlePlayback = useCallback(async (action: "play" | "pause" | "stop") => {
    try {
      await invoke("set_engine_playback", { action });
      setPlaybackState(action === "play" ? "playing" : action === "pause" ? "paused" : "stopped");
      if (action === "stop") await refreshScene();
    } catch (err) {
      console.error("Failed to set playback state:", err);
    }
  }, [refreshScene]);

  const handleOpenScript = useCallback(async (path: string) => {
    try {
      const content = await invoke<string>("get_script", { path });
      setOpenScript({ path, content });
    } catch (err) {
      console.error("Failed to open script:", err);
    }
  }, []);

  const handleOpenScene = useCallback(async (relativePath: string) => {
    if (!projectPath) return;
    try {
      await invoke("open_scene_file", { projectPath, relativePath });
      localStorage.setItem(`sindri_last_scene:${projectPath}`, relativePath);
      setSelectedId(null);
      setSelectedComponent(null);
      setLeftTab("scene");
      undoStack.current = [];
      redoStack.current = [];
      setUndoLabels([]);
      setRedoLabels([]);
      syncHistoryDepths();
      setSceneDirty(false);
      await refreshScene();
    } catch (err) {
      console.error("Failed to open scene:", err);
    }
  }, [projectPath, refreshScene, syncHistoryDepths]);

  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const isTextInput = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA" || target?.getAttribute("contenteditable") === "true";

      if ((e.metaKey || e.ctrlKey) && (e.key === "k" || e.key === "K")) {
        e.preventDefault();
        setCmdKOpen(o => !o);
        return;
      }
      if (e.key === "Escape" && cmdKOpen) {
        setCmdKOpen(false);
        return;
      }
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        await saveScene();
        return;
      }
      if (!isTextInput && (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "z") {
        e.preventDefault();
        if (e.shiftKey) await redoTransform();
        else await undoTransform();
        return;
      }
      if (!isTextInput && (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "y") {
        e.preventDefault();
        await redoTransform();
        return;
      }
      if (e.key === "Delete" && selectedId !== null && !isTextInput) {
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
  }, [selectedId, handleSceneChange, redoTransform, saveScene, undoTransform, cmdKOpen]);

  if (!projectPath) {
    return (
      <ContextMenuProvider>
        <div style={{ width: "100%", height: "100%" }}>
          <WelcomeScreen onOpen={(dir, name) => { sceneRestoredRef.current = false; setProjectPath(dir); setProjectName(name); }} />
        </div>
      </ContextMenuProvider>
    );
  }

  const selectedProviderStatus = providerStatuses.find(s => s.provider === selectedProvider);
  const providerConfigured = selectedProvider === "ollama" ? ollamaReady : selectedProviderStatus?.configured === true;
  const resolvedModelConfig: AiModelConfig = {
    assistant: modelConfig.assistant || (selectedProvider === "ollama" ? preferredOllamaModel(ollamaModels, "assistant") : selectedProviderStatus?.defaultModel ?? null),
    code: modelConfig.code || (selectedProvider === "ollama" ? preferredOllamaModel(ollamaModels, "code") : selectedProviderStatus?.defaultModel ?? null),
    fast: modelConfig.fast || (selectedProvider === "ollama" ? preferredOllamaModel(ollamaModels, "fast") : selectedProviderStatus?.defaultModel ?? null),
    vision: modelConfig.vision || (selectedProvider === "ollama" ? preferredOllamaModel(ollamaModels, "vision") : selectedProviderStatus?.defaultModel ?? null),
  };
  const effectiveModel = resolvedModelConfig.assistant || "";
  const aiStatusDot = providerConfigured ? "var(--amber)" : "var(--ink-4)";
  const aiStatusText = providerConfigured
    ? `${selectedProvider} · ${effectiveModel || "ready"}`
    : `${selectedProvider} not configured`;

  return (
    <ContextMenuProvider>
      <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--paper)", color: "var(--ink)" }}>
        <Topbar
          sceneName={scene?.name ?? ""}
          projectName={projectName}
          sceneDirty={sceneDirty}
          playbackState={playbackState}
          engineReady={engineReady}
          onPlayback={handlePlayback}
          onOpenCmdK={() => setCmdKOpen(true)}
          aiStatusDot={aiStatusDot}
          aiStatusText={aiStatusText}
          activeTool={activeTool}
          setActiveTool={setActiveTool}
          canUndo={undoDepth > 0}
          canRedo={redoDepth > 0}
          onUndo={undoTransform}
          onRedo={redoTransform}
          onOpenAiSettings={() => setAiSettingsOpen(true)}
          onOpenSettings={() => setSettingsOpen(true)}
          resolution={projectSettings ? `${projectSettings.resolution_width} × ${projectSettings.resolution_height}` : "1280 × 720"}
        />

        <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
          {/* Left panel */}
          <div style={{
            width: "var(--panel-w-left)",
            minWidth: "var(--panel-w-left)",
            background: "var(--paper)",
            borderRight: "1px solid var(--rule)",
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}>
            {/* Tabs */}
            <div style={{
              display: "flex",
              height: "42px",
              padding: "0 16px",
              borderBottom: "1px solid var(--rule)",
              alignItems: "flex-end",
              gap: "20px",
              flexShrink: 0,
            }}>
              {(["scene", "files", "history"] as const).map(tab => (
                <button
                  key={tab}
                  onClick={() => setLeftTab(tab)}
                  style={{
                    paddingBottom: "10px",
                    paddingTop: "10px",
                    fontSize: "12.5px",
                    fontFamily: "var(--font-ui)",
                    color: leftTab === tab ? "var(--ink)" : "var(--ink-3)",
                    fontWeight: leftTab === tab ? 500 : 400,
                    borderBottom: leftTab === tab ? "2px solid var(--ink)" : "2px solid transparent",
                    borderTop: "none", borderLeft: "none", borderRight: "none",
                    marginBottom: "-1px",
                    background: "none",
                    cursor: "pointer",
                    textTransform: "capitalize",
                  }}
                >{tab}</button>
              ))}
            </div>

            {leftTab === "scene" && (
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
            )}
            {leftTab === "files" && (
              <FileBrowser
                projectPath={projectPath}
                onOpenScript={handleOpenScript}
                onOpenScene={handleOpenScene}
                onFilesChange={setProjectFiles}
              />
            )}
            {leftTab === "history" && (
              <HistoryPanel undoLabels={undoLabels} redoLabels={redoLabels} onUndo={undoTransform} onRedo={redoTransform} />
            )}
          </div>

          {/* Center column */}
          <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", minWidth: 0 }}>
            <Viewport
              scene={scene}
              selectedId={selectedId}
              onSelect={setSelectedId}
              activeTool={activeTool}
              onTransformCommit={handleTransformCommit}
              onColliderCommit={handleColliderCommit}
              engineReady={engineReady}
              isPlaying={playbackState === "playing"}
              resolution={projectSettings ? `${projectSettings.resolution_width} × ${projectSettings.resolution_height}` : "1280 × 720"}
            />
            <ScriptEditor
              openScript={openScript}
              onClose={() => setOpenScript(null)}
            />
          </div>

          {/* Right panel — Proposals Lane (when active) or Inspector */}
          <div style={{
            width: "var(--panel-w-right)",
            minWidth: "var(--panel-w-right)",
            background: "var(--paper)",
            borderLeft: "1px solid var(--rule)",
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}>
            {pendingProposal ? (
              <ProposalsLane
                proposal={pendingProposal}
                onSceneChange={handleSceneChange}
                onClose={() => setPendingProposal(null)}
              />
            ) : (
              <Inspector
                entity={selectedEntity}
                selectedComponent={selectedComponent}
                onSelectComponent={type => setSelectedComponent(type)}
                onSceneChange={handleSceneChange}
                onOpenScript={handleOpenScript}
                suggestionModel={suggestionModel}
                projectFiles={projectFiles}
                projectPath={projectPath}
                onAskAI={(prompt, mode) => {
                  setCmdKInit(mode === "send" ? { message: prompt } : { input: prompt });
                  setCmdKOpen(true);
                }}
              />
            )}
          </div>
        </div>

        <Statusbar
          engineReady={engineReady}
          ollamaReady={ollamaReady}
          scene={scene}
          selectedModel={resolvedModelConfig.assistant}
          runtimeErrors={runtimeErrors}
          suggestionModelPulling={suggestionModelPulling}
          suggestionModel={suggestionModel}
        />
      </div>

      {cmdKOpen && (
        <CmdK
          scene={scene}
          projectPath={projectPath}
          openScript={openScript}
          modelConfig={resolvedModelConfig}
          selectedProvider={selectedProvider}
          projectFiles={projectFiles}
          runtimeErrors={runtimeErrors}
          onSceneChange={handleSceneChange}
          onClose={() => { setCmdKOpen(false); setCmdKInit(null); }}
          selectedEntity={selectedEntity}
          initialMessage={cmdKInit?.message}
          initialInput={cmdKInit?.input}
          onProposalReady={(proposal: ProposalData) => {
            setPendingProposal(proposal);
            setCmdKOpen(false);
            setCmdKInit(null);
          }}
        />
      )}

      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        projectPath={projectPath}
        engineReady={engineReady}
      />

      {aiSettingsOpen && (
        <AiSettingsModal
          provider={selectedProvider}
          setProvider={provider => {
            setSelectedProvider(provider);
            const status = providerStatuses.find(s => s.provider === provider);
            setModelConfig(provider === "ollama"
              ? defaultModelConfig(ollamaModels)
              : {
                  assistant: status?.defaultModel ?? "",
                  code: status?.defaultModel ?? "",
                  fast: status?.defaultModel ?? "",
                  vision: status?.defaultModel ?? "",
                });
          }}
          modelConfig={modelConfig}
          setModelForRole={(role, model) => setModelConfig(current => ({ ...current, [role]: model }))}
          ollamaModels={ollamaModels}
          statuses={providerStatuses}
          onRefresh={refreshProviderStatuses}
          onClose={() => setAiSettingsOpen(false)}
        />
      )}

      <style>{`
        @keyframes v3dot {
          0%, 100% { opacity: 0.25; transform: translateY(0); }
          50% { opacity: 1; transform: translateY(-3px); }
        }
      `}</style>
    </ContextMenuProvider>
  );
}

// ─── Topbar ────────────────────────────────────────────────────────────────

interface TopbarProps {
  sceneName: string;
  projectName: string;
  sceneDirty: boolean;
  playbackState: PlaybackState;
  engineReady: boolean;
  onPlayback: (action: "play" | "pause" | "stop") => void;
  onOpenCmdK: () => void;
  aiStatusDot: string;
  aiStatusText: string;
  activeTool: ActiveTool;
  setActiveTool: (t: ActiveTool) => void;
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
  onOpenAiSettings: () => void;
  onOpenSettings: () => void;
  resolution: string;
}

function Topbar({
  sceneName, projectName, sceneDirty,
  playbackState, engineReady, onPlayback,
  onOpenCmdK, aiStatusDot, aiStatusText,
  activeTool, setActiveTool,
  canUndo, canRedo, onUndo, onRedo,
  onOpenAiSettings, onOpenSettings, resolution,
}: TopbarProps) {
  const tools: { key: ActiveTool; icon: string; label: string }[] = [
    { key: "select",   icon: "↖", label: "Select" },
    { key: "move",     icon: "✥", label: "Move" },
    { key: "scale",    icon: "⤢", label: "Scale" },
    { key: "rotate",   icon: "↻", label: "Rotate" },
    { key: "collider", icon: "⬡", label: "Edit Collider" },
  ];

  return (
    <header style={{
      height: "var(--header-h)",
      display: "grid",
      gridTemplateColumns: "380px 1fr 300px",
      borderBottom: "1px solid var(--rule-2)",
      background: "var(--paper)",
      alignItems: "center",
      flexShrink: 0,
    }}>
      {/* Left: Logo + breadcrumb */}
      <div style={{
        display: "flex", alignItems: "center",
        padding: "0 18px", gap: "12px",
        height: "100%", overflow: "hidden", minWidth: 0,
      }}>
        <ForgeLogo size={22} />
        <span style={{
          fontFamily: "var(--font-ui)", fontWeight: 700, fontSize: "18px",
          letterSpacing: "0.14em", color: "var(--ink)",
          display: "flex", alignItems: "center", gap: "1px",
          flex: "none",
        }}>
          S<span style={{ color: "var(--amber)" }}>I</span>NDRI
        </span>
        <div style={{ width: "1px", height: "20px", background: "var(--rule-2)", flex: "none" }} />
        <div style={{
          fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-3)",
          display: "flex", alignItems: "baseline", gap: "6px",
          overflow: "hidden", minWidth: 0,
          whiteSpace: "nowrap",
        }}>
          <span style={{ flex: "none" }}>scenes</span>
          <span style={{ color: "var(--ink-4)", flex: "none" }}>/</span>
          <span style={{
            color: "var(--ink)",
            overflow: "hidden", textOverflow: "ellipsis",
            minWidth: 0, flex: "1 1 auto",
          }}>
            {sceneName || projectName || "untitled"}{sceneDirty ? " *" : ""}
          </span>
        </div>
      </div>

      {/* Center: Cmd-K + tools + run controls */}
      <div style={{
        display: "flex", alignItems: "center",
        padding: "0 18px", gap: "12px", height: "100%",
      }}>
        {/* Cmd-K input */}
        <button
          onClick={onOpenCmdK}
          style={{
            display: "flex", alignItems: "center", gap: "10px",
            height: "32px", padding: "0 14px",
            background: "var(--paper-2)",
            border: "1px solid var(--rule)",
            color: "var(--ink-3)",
            fontSize: "13px", fontFamily: "var(--font-ui)",
            flex: 1, maxWidth: "380px",
            cursor: "text",
          }}
        >
          <SparkleIcon size={14} />
          <span style={{ flex: 1, textAlign: "left" }}>Ask Sindri to build something…</span>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>
            <KbdKey>Ctrl</KbdKey><KbdKey>K</KbdKey>
          </span>
        </button>

        {/* Tool buttons */}
        <div style={{ display: "flex", gap: "2px" }}>
          {tools.map(t => (
            <button key={t.key} title={t.label} onClick={() => setActiveTool(t.key)} style={{
              width: "28px", height: "28px",
              display: "inline-flex", alignItems: "center", justifyContent: "center",
              background: activeTool === t.key ? "var(--paper-3)" : "transparent",
              border: activeTool === t.key ? "1px solid var(--rule-2)" : "1px solid transparent",
              color: activeTool === t.key ? "var(--ink)" : "var(--ink-3)",
              fontSize: "13px", cursor: "pointer",
              fontFamily: "var(--font-mono)",
            }}>{t.icon}</button>
          ))}
        </div>

        <div style={{ width: "1px", height: "18px", background: "var(--rule-2)" }} />

        {/* Undo/Redo */}
        <button title="Undo (Ctrl+Z)" onClick={canUndo ? onUndo : undefined} style={iconBtnStyle(canUndo)}>↶</button>
        <button title="Redo (Ctrl+Y)" onClick={canRedo ? onRedo : undefined} style={iconBtnStyle(canRedo)}>↷</button>

        <div style={{ width: "1px", height: "18px", background: "var(--rule-2)" }} />

        {/* Run controls */}
        <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
          <RunBtn
            title={playbackState === "paused" ? "Resume" : "Play"}
            disabled={!engineReady || playbackState === "playing"}
            variant="play"
            onClick={() => onPlayback("play")}
          />
          <RunBtn
            title="Pause"
            disabled={!engineReady || playbackState !== "playing"}
            variant="pause"
            onClick={() => onPlayback("pause")}
          />
          <RunBtn
            title="Stop"
            disabled={!engineReady || playbackState === "stopped"}
            variant="stop"
            onClick={() => onPlayback("stop")}
          />
        </div>
      </div>

      {/* Right: resolution + settings + AI */}
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "flex-end",
        padding: "0 16px", gap: "8px", height: "100%",
      }}>
        {/* Compose link */}
        <button
          onClick={onOpenCmdK}
          style={{
            display: "inline-flex", alignItems: "center", gap: "6px",
            fontFamily: "var(--font-ui)", fontSize: "14px", color: "var(--amber)",
            background: "none", border: "none", cursor: "pointer",
          }}
        >
          <SparkleIcon size={13} /> Compose
        </button>

        <div style={{ width: "1px", height: "18px", background: "var(--rule-2)" }} />

        {/* Resolution */}
        <button
          onClick={onOpenSettings}
          title="Project settings"
          style={{
            fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)",
            background: "none", border: "none", cursor: "pointer", padding: "0 4px",
          }}
        >
          {resolution}
        </button>

        {/* Settings button */}
        <button
          onClick={onOpenSettings}
          title="Project & editor settings"
          style={{
            width: "28px", height: "28px",
            background: "transparent",
            border: "1px solid var(--rule)",
            color: "var(--ink-3)",
            cursor: "pointer",
            fontFamily: "var(--font-mono)",
            fontSize: "14px",
          }}
        >⊞</button>

        <div style={{ width: "1px", height: "18px", background: "var(--rule-2)" }} />

        {/* AI status pill */}
        <div style={{
          display: "flex", alignItems: "center", gap: "8px",
          fontSize: "11.5px", color: "var(--ink-3)",
          fontFamily: "var(--font-mono)",
          padding: "5px 10px",
          border: "1px solid var(--rule)",
          background: "var(--paper-2)",
          maxWidth: "200px",
          whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis",
        }}>
          <span style={{ width: "6px", height: "6px", background: aiStatusDot, flexShrink: 0 }} />
          <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {aiStatusText}
          </span>
        </div>
        <button
          onClick={onOpenAiSettings}
          title="AI provider settings"
          style={{
            width: "28px", height: "28px",
            background: "transparent",
            border: "1px solid var(--rule)",
            color: "var(--ink-3)",
            cursor: "pointer",
            fontFamily: "var(--font-mono)",
          }}
        >⚙</button>
      </div>
    </header>
  );
}

function iconBtnStyle(enabled: boolean): CSSProperties {
  return {
    width: "26px", height: "26px",
    display: "inline-flex", alignItems: "center", justifyContent: "center",
    background: "transparent",
    border: "none",
    color: enabled ? "var(--ink-2)" : "var(--ink-4)",
    fontFamily: "var(--font-mono)",
    fontSize: "14px",
    cursor: enabled ? "pointer" : "default",
  };
}

function RunBtn({ title, disabled, variant, onClick }: {
  title: string;
  disabled: boolean;
  variant: "play" | "pause" | "stop";
  onClick: () => void;
}) {
  const isPlay = variant === "play";
  return (
    <button
      onClick={disabled ? undefined : onClick}
      title={title}
      style={{
        width: "34px", height: "28px",
        display: "inline-flex", alignItems: "center", justifyContent: "center",
        cursor: disabled ? "default" : "pointer",
        background: isPlay && !disabled ? "var(--ink)" : "transparent",
        color: disabled
          ? "var(--ink-4)"
          : isPlay ? "var(--paper)" : "var(--ink-3)",
        border: isPlay && !disabled
          ? "1px solid var(--ink)"
          : "1px solid var(--rule-2)",
      }}
    >
      {variant === "play" && (
        <span style={{
          width: 0, height: 0,
          borderTop: "5px solid transparent",
          borderBottom: "5px solid transparent",
          borderLeft: `8px solid currentColor`,
          display: "inline-block",
        }} />
      )}
      {variant === "pause" && (
        <span style={{ display: "flex", gap: "2px" }}>
          <span style={{ width: "3px", height: "10px", background: "currentColor", display: "inline-block" }} />
          <span style={{ width: "3px", height: "10px", background: "currentColor", display: "inline-block" }} />
        </span>
      )}
      {variant === "stop" && (
        <span style={{ width: "9px", height: "9px", background: "currentColor", display: "inline-block" }} />
      )}
    </button>
  );
}

function AiSettingsModal({
  provider,
  setProvider,
  modelConfig,
  setModelForRole,
  ollamaModels,
  statuses,
  onRefresh,
  onClose,
}: {
  provider: AiProvider;
  setProvider: (provider: AiProvider) => void;
  modelConfig: AiModelConfig;
  setModelForRole: (role: AiModelRole, model: string | null) => void;
  ollamaModels: string[];
  statuses: AiProviderStatus[];
  onRefresh: () => Promise<void>;
  onClose: () => void;
}) {
  const [apiKey, setApiKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [consent, setConsent] = useState(() => localStorage.getItem("sindri_cloud_ai_consent") === "accepted");
  const activeStatus = statuses.find(s => s.provider === provider);
  const cloud = provider !== "ollama";

  const acceptConsent = () => {
    localStorage.setItem("sindri_cloud_ai_consent", "accepted");
    setConsent(true);
  };

  const saveKey = async () => {
    if (!cloud) return;
    setBusy(true);
    setMessage("");
    try {
      await invoke("save_ai_api_key", { provider, apiKey });
      setApiKey("");
      await onRefresh();
      setMessage(`${provider} key saved to OS keychain.`);
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const clearKey = async () => {
    if (!cloud) return;
    setBusy(true);
    setMessage("");
    try {
      await invoke("clear_ai_api_key", { provider });
      await onRefresh();
      setMessage(`${provider} key cleared.`);
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const testProvider = async () => {
    setBusy(true);
    setMessage("");
    try {
      await invoke("test_ai_provider", {
        provider,
        model: modelConfig.assistant
          ?? (provider === "ollama" ? preferredOllamaModel(ollamaModels, "assistant") : activeStatus?.defaultModel),
      });
      await onRefresh();
      setMessage(`${provider} responded successfully.`);
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const providerCard = (id: AiProvider, title: string, detail: string) => {
    const status = statuses.find(s => s.provider === id);
    const selected = provider === id;
    return (
      <button
        key={id}
        onClick={() => {
          setProvider(id);
          setMessage("");
        }}
        style={{
          textAlign: "left",
          border: `1px solid ${selected ? "var(--amber)" : "var(--rule-2)"}`,
          background: selected ? "rgba(240,192,80,0.08)" : "var(--paper)",
          color: selected ? "var(--ink)" : "var(--ink-2)",
          padding: "12px",
          cursor: "pointer",
          display: "grid",
          gap: "6px",
          fontFamily: "var(--font-ui)",
        }}
      >
        <span style={{ display: "flex", justifyContent: "space-between", gap: "10px" }}>
          <strong>{title}</strong>
          <span style={{
            fontFamily: "var(--font-mono)",
            color: status?.configured ? "var(--moss)" : "var(--ink-4)",
            fontSize: "10.5px",
          }}>
            {status?.configured ? "configured" : id === "ollama" ? "offline" : "no key"}
          </span>
        </span>
        <span style={{ color: "var(--ink-3)", fontSize: "12px", lineHeight: 1.4 }}>{detail}</span>
      </button>
    );
  };

  return (
    <div
      onClick={onClose}
      style={{
        position: "fixed", inset: 0,
        background: "rgba(0,0,0,0.64)",
        zIndex: 260,
        display: "flex", alignItems: "center", justifyContent: "center",
        padding: "32px",
      }}
    >
      <div
        onClick={e => e.stopPropagation()}
        style={{
          width: "760px",
          maxWidth: "calc(100vw - 48px)",
          maxHeight: "calc(100vh - 48px)",
          overflow: "auto",
          background: "var(--paper)",
          border: "1px solid var(--rule-2)",
          boxShadow: "0 24px 80px rgba(0,0,0,0.5)",
        }}
      >
        <div style={{
          height: "52px",
          display: "flex", alignItems: "center",
          padding: "0 18px",
          borderBottom: "1px solid var(--rule)",
          gap: "12px",
        }}>
          <SparkleIcon size={15} />
          <div style={{ flex: 1 }}>
            <div style={{ color: "var(--ink)", fontSize: "15px" }}>AI Provider</div>
            <div style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10.5px" }}>
              local-first by default · cloud is BYOK and opt-in
            </div>
          </div>
          <button onClick={onClose} style={{ background: "transparent", border: "none", color: "var(--ink-3)", cursor: "pointer", fontSize: "18px" }}>×</button>
        </div>

        <div style={{ padding: "18px", display: "grid", gap: "16px" }}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "10px" }}>
            {providerCard("ollama", "Ollama", "Offline local models on localhost. No API key and no external network calls.")}
            {providerCard("openai", "OpenAI", "BYOK cloud models for stronger code, vision, and reasoning.")}
            {providerCard("anthropic", "Anthropic", "BYOK Claude models for long-context planning and code review.")}
          </div>

          {cloud && (
            <div style={{
              border: `1px solid ${consent ? "var(--rule-2)" : "var(--amber)"}`,
              background: consent ? "var(--paper-2)" : "rgba(240,192,80,0.08)",
              padding: "14px",
              display: "grid",
              gap: "10px",
              color: "var(--ink-2)",
              lineHeight: 1.5,
            }}>
              <strong style={{ color: "var(--ink)" }}>Cloud AI privacy notice</strong>
              <p>
                When you use OpenAI or Anthropic, enabled context may be sent to that external provider:
                scene JSON, open Lua scripts, viewport screenshots, runtime errors, chat history, and your prompt.
                Sindri is not responsible for personal, private, proprietary, or sensitive data you choose to send.
                Use cloud providers at your own discretion and under that provider&apos;s terms.
              </p>
              {!consent && (
                <button onClick={acceptConsent} style={{
                  justifySelf: "start",
                  background: "var(--amber)",
                  color: "var(--paper)",
                  border: "1px solid var(--amber)",
                  padding: "7px 12px",
                  cursor: "pointer",
                  fontFamily: "var(--font-ui)",
                }}>
                  I understand and want cloud AI available
                </button>
              )}
            </div>
          )}

          <div style={{ display: "grid", gridTemplateColumns: cloud ? "1fr 1fr" : "1fr", gap: "14px", alignItems: "start" }}>
            <div style={{ display: "grid", gap: "8px" }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>model roles</span>
              {AI_MODEL_ROLES.map(({ role, title, detail }) => (
                <ModelRolePicker
                  key={role}
                  provider={provider}
                  role={role}
                  title={title}
                  detail={detail}
                  value={modelConfig[role]}
                  models={ollamaModels}
                  defaultModel={provider === "ollama"
                    ? preferredOllamaModel(ollamaModels, role)
                    : activeStatus?.defaultModel ?? ""}
                  onChange={model => setModelForRole(role, model)}
                />
              ))}
              {provider === "ollama" && (
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "10.5px", color: "var(--ink-4)" }}>
                  {ollamaModels.length > 0
                    ? `from \`ollama list\` · ${ollamaModels.length} installed`
                    : "No installed models found from `ollama list`."}
                </span>
              )}
            </div>

            {cloud && (
              <label style={{ display: "grid", gap: "6px", opacity: consent ? 1 : 0.45 }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>api key</span>
                <input
                  value={apiKey}
                  disabled={!consent}
                  onChange={e => setApiKey(e.target.value)}
                  type="password"
                  placeholder={activeStatus?.configured ? "saved in OS keychain" : `${provider.toUpperCase()} API key`}
                  style={{
                    background: "var(--paper-2)",
                    border: "1px solid var(--rule-2)",
                    color: "var(--ink)",
                    padding: "8px 10px",
                    fontFamily: "var(--font-mono)",
                    outline: "none",
                  }}
                />
              </label>
            )}
          </div>

          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            {cloud && (
              <>
                <button disabled={!consent || busy || !apiKey.trim()} onClick={saveKey} style={settingsButtonStyle(!consent || busy || !apiKey.trim(), true)}>save key</button>
                <button disabled={!consent || busy || !activeStatus?.configured} onClick={clearKey} style={settingsButtonStyle(!consent || busy || !activeStatus?.configured)}>clear key</button>
              </>
            )}
            <button disabled={busy || (cloud && (!consent || !activeStatus?.configured))} onClick={testProvider} style={settingsButtonStyle(busy || (cloud && (!consent || !activeStatus?.configured)))}>
              test provider
            </button>
            <div style={{ flex: 1 }} />
            <button onClick={onClose} style={settingsButtonStyle(false, true)}>done</button>
          </div>

          {message && (
            <div style={{
              fontFamily: "var(--font-mono)",
              fontSize: "11px",
              color: message.includes("success") || message.includes("saved") ? "var(--moss)" : "var(--ink-3)",
              borderTop: "1px solid var(--rule)",
              paddingTop: "10px",
              whiteSpace: "pre-wrap",
            }}>
              {message}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function settingsButtonStyle(disabled: boolean, primary = false): CSSProperties {
  return {
    background: disabled ? "transparent" : primary ? "var(--ink)" : "var(--paper-2)",
    border: "1px solid var(--rule-2)",
    color: disabled ? "var(--ink-4)" : primary ? "var(--paper)" : "var(--ink-2)",
    padding: "7px 12px",
    cursor: disabled ? "default" : "pointer",
    fontFamily: "var(--font-ui)",
    fontSize: "12px",
  };
}

function ModelRolePicker({
  provider,
  role,
  title,
  detail,
  value,
  models,
  defaultModel,
  onChange,
}: {
  provider: AiProvider;
  role: AiModelRole;
  title: string;
  detail: string;
  value: string | null;
  models: string[];
  defaultModel: string | null;
  onChange: (model: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const [menuRect, setMenuRect] = useState<{ left: number; top: number; width: number } | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const local = provider === "ollama";
  const hasLocalModels = local && models.length > 0;
  const resolvedDefault = defaultModel || null;
  const label = value && (!local || models.includes(value))
    ? value
    : resolvedDefault ? `auto · ${resolvedDefault}` : "not configured";

  useEffect(() => {
    if (!open) return;

    const updateRect = () => {
      const rect = buttonRef.current?.getBoundingClientRect();
      if (!rect) return;
      setMenuRect({ left: rect.left, top: rect.bottom + 4, width: rect.width });
    };

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (buttonRef.current?.contains(target) || menuRef.current?.contains(target)) return;
      setOpen(false);
    };

    updateRect();
    document.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("resize", updateRect);
    window.addEventListener("scroll", updateRect, true);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("resize", updateRect);
      window.removeEventListener("scroll", updateRect, true);
    };
  }, [open]);

  return (
    <div style={{
      display: "grid",
      gridTemplateColumns: "110px 1fr",
      gap: "10px",
      alignItems: "center",
      border: "1px solid var(--rule)",
      background: "var(--paper-2)",
      padding: "8px",
    }}>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontFamily: "var(--font-ui)", color: "var(--ink)", fontSize: "12px" }}>{title}</div>
        <div style={{ fontFamily: "var(--font-mono)", color: "var(--ink-4)", fontSize: "10px", lineHeight: 1.35 }}>{detail}</div>
      </div>

      {hasLocalModels ? (
        <div
          tabIndex={0}
          onKeyDown={event => {
            if (event.key === "Escape") setOpen(false);
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              setOpen(current => !current);
            }
          }}
          style={{ position: "relative", outline: "none" }}
        >
          <button
            ref={buttonRef}
            type="button"
            onClick={() => setOpen(current => !current)}
            style={{
              width: "100%",
              height: "32px",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              gap: "12px",
              background: "var(--paper)",
              border: `1px solid ${open ? "var(--amber)" : "var(--rule-2)"}`,
              color: "var(--ink)",
              padding: "0 10px",
              fontFamily: "var(--font-mono)",
              fontSize: "11px",
              cursor: "pointer",
              boxShadow: open ? "0 0 0 2px rgba(240,192,80,0.08)" : "none",
            }}
          >
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{label}</span>
            <span style={{
              width: 0,
              height: 0,
              borderLeft: "4px solid transparent",
              borderRight: "4px solid transparent",
              borderTop: "5px solid var(--ink-3)",
              transform: open ? "rotate(180deg)" : "none",
            }} />
          </button>

          {open && menuRect && createPortal(
            <div
              ref={menuRef}
              style={{
                position: "fixed",
                zIndex: 1000,
                left: `${menuRect.left}px`,
                top: `${menuRect.top}px`,
                width: `${menuRect.width}px`,
                maxHeight: "190px",
                overflowY: "auto",
                background: "var(--paper)",
                border: "1px solid var(--rule-2)",
                boxShadow: "0 14px 34px rgba(0,0,0,0.45)",
                padding: "4px",
              }}
            >
              <button
                type="button"
                onMouseDown={event => event.preventDefault()}
                onClick={() => {
                  onChange(null);
                  setOpen(false);
                }}
                style={modelOptionStyle(!value || !models.includes(value))}
              >
                <span>auto</span>
                <span style={{ color: "var(--ink-4)" }}>{resolvedDefault ?? "none"}</span>
              </button>
              {models.map(name => (
                <button
                  key={`${role}:${name}`}
                  type="button"
                  onMouseDown={event => event.preventDefault()}
                  onClick={() => {
                    onChange(name);
                    setOpen(false);
                  }}
                  style={modelOptionStyle(value === name)}
                >
                  <span>{name}</span>
                  {value === name && <span style={{ color: "var(--amber)" }}>selected</span>}
                </button>
              ))}
            </div>,
            document.body
          )}
        </div>
      ) : (
        <input
          value={value ?? ""}
          disabled={local}
          onChange={event => onChange(event.target.value.trim() || null)}
          placeholder={local ? "No local Ollama models found" : resolvedDefault ?? "model name"}
          style={{
            height: "32px",
            background: "var(--paper)",
            border: "1px solid var(--rule-2)",
            color: local ? "var(--ink-4)" : "var(--ink)",
            padding: "0 10px",
            fontFamily: "var(--font-mono)",
            fontSize: "11px",
            outline: "none",
          }}
        />
      )}
    </div>
  );
}

function modelOptionStyle(selected: boolean): CSSProperties {
  return {
    width: "100%",
    minHeight: "30px",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: "12px",
    background: selected ? "rgba(240,192,80,0.08)" : "transparent",
    border: `1px solid ${selected ? "var(--amber)" : "transparent"}`,
    color: selected ? "var(--ink)" : "var(--ink-2)",
    padding: "6px 8px",
    cursor: "pointer",
    fontFamily: "var(--font-mono)",
    fontSize: "11px",
    textAlign: "left",
  };
}

// ─── Forge logo mark ───────────────────────────────────────────────────────

function ForgeLogo({ size = 22 }: { size?: number }) {
  const s = size;
  // Point-up hexagon helper — vertices at top
  const hex = (r: number) => {
    const pts: string[] = [];
    for (let i = 0; i < 6; i++) {
      const a = (Math.PI / 180) * (60 * i - 90);
      pts.push(`${s / 2 + r * Math.cos(a)},${s / 2 + r * Math.sin(a)}`);
    }
    return pts.join(" ");
  };
  const r1 = s * 0.41;   // outer hex radius
  const r2 = s * 0.22;   // inner hex radius
  const r3 = s * 0.088;  // ember hex radius

  return (
    <svg width={s} height={s} viewBox={`0 0 ${s} ${s}`} style={{ flexShrink: 0 }}>
      <polygon points={hex(r1)} fill="none" stroke="var(--ink-2)" strokeWidth="1.6" />
      <polygon points={hex(r2)} fill="none" stroke="var(--ink-2)" strokeWidth="1.2" />
      <polygon points={hex(r3)} fill="var(--amber)" />
    </svg>
  );
}

// ─── Sparkle icon ──────────────────────────────────────────────────────────

function SparkleIcon({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" style={{ flexShrink: 0 }}>
      <path d="M8 1v14M1 8h14M3.5 3.5l9 9M12.5 3.5l-9 9" stroke="var(--amber)" strokeWidth="1.4" strokeLinecap="square" />
    </svg>
  );
}

// ─── Keyboard key ──────────────────────────────────────────────────────────

export function KbdKey({ children }: { children: React.ReactNode; }) {
  return (
    <span style={{
      display: "inline-block", padding: "1px 5px",
      border: "1px solid var(--rule-2)",
      fontFamily: "var(--font-mono)", fontSize: "10.5px",
      color: "var(--ink-4)", margin: "0 1px",
    }}>{children}</span>
  );
}

// ─── History panel ─────────────────────────────────────────────────────────

function HistoryPanel({ undoLabels, redoLabels, onUndo, onRedo }: {
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
          {/* Redo stack (future, greyed) — shown top-to-bottom = oldest redo last */}
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

          {/* Current state marker */}
          <div style={{
            display: "flex", alignItems: "center", gap: "8px",
            padding: "5px 16px",
            borderTop: "1px solid var(--rule)", borderBottom: "1px solid var(--rule)",
          }}>
            <span style={{ width: "10px", height: "10px", background: "var(--amber)", flexShrink: 0 }} />
            <span style={{ fontSize: "11px", color: "var(--amber)", fontFamily: "var(--font-mono)" }}>current</span>
          </div>

          {/* Undo stack (past) — most recent first */}
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

// ─── Status bar ────────────────────────────────────────────────────────────

interface StatusbarProps {
  engineReady: boolean;
  ollamaReady: boolean;
  scene: Scene | null;
  selectedModel: string | null;
  runtimeErrors: string[];
  suggestionModelPulling: boolean;
  suggestionModel: string | null;
}

function Statusbar({ engineReady, ollamaReady, scene, selectedModel, runtimeErrors, suggestionModelPulling, suggestionModel }: StatusbarProps) {
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
