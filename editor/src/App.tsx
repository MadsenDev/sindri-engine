import { useState, useEffect, useCallback, useRef } from "react";
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
import type { SettingsTab } from "./components/SettingsModal";
import Topbar from "./components/Topbar";
import HistoryPanel from "./components/HistoryPanel";
import Statusbar from "./components/Statusbar";


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
  prefab_source?: string;
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
  z_index: number;
}

export type Component =
  | { type: "Transform"; x: number; y: number; scale_x: number; scale_y: number; rotation: number; z_index?: number; pivot_x?: number; pivot_y?: number }
  | { type: "Sprite"; texture_path: string; width: number; height: number; flip_x: boolean; flip_y: boolean; color: [number, number, number, number] }
  | { type: "AnimatedSprite"; texture_path: string; cols: number; rows: number; width: number; height: number; flip_x: boolean; flip_y: boolean; tint: [number, number, number, number]; clips: AnimClip[]; default_clip: string; margin?: number; spacing?: number }
  | { type: "PhysicsBody"; body_type: "Dynamic" | "Kinematic" | "Fixed"; lock_rotation: boolean; linear_damping: number; angular_damping: number; collision_layer: number; collision_mask: number; gravity_scale: number }
  | { type: "Collider"; width: number; height: number; offset_x: number; offset_y: number; is_trigger: boolean; block_pathfinding: boolean }
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
  script_new_contents: { path: string; content: string }[];
}

export interface ProposalData {
  prompt: string;
  summary: string;
  changes: ProposalChange[];
}


export type ActiveTool = "select" | "move" | "scale" | "rotate" | "collider";

export interface TilemapEdit {
  paletteIdx: number;
  tileIdx: number;
  layerIdx: number;
  mode: "draw" | "erase" | "collision" | "stamp";
  selectedPrefabPath: string | null;
}
type PlaybackState = "stopped" | "playing" | "paused";
type TransformSnapshot = TransformChange["before"];
interface EngineStatus {
  status: string;
  model: string;
  paused: boolean;
  playback?: string | null;
  error_count?: number | null;
}

type AiProvider = "ollama" | "openai" | "anthropic" | "openrouter";

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
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState<SettingsTab>("project");
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
  const [tilemapEdit, setTilemapEdit] = useState<TilemapEdit>({
    paletteIdx: 0, tileIdx: 0, layerIdx: 0, mode: "draw", selectedPrefabPath: null,
  });
  const [pendingProposal, setPendingProposal] = useState<ProposalData | null>(null);
  const [resolvedProposalKeys, setResolvedProposalKeys] = useState<Set<string>>(new Set());
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
  const resolvedModelConfig: AiModelConfig = {
    assistant: modelConfig.assistant || (selectedProvider === "ollama" ? preferredOllamaModel(ollamaModels, "assistant") : selectedProviderStatus?.defaultModel ?? null),
    code: modelConfig.code || (selectedProvider === "ollama" ? preferredOllamaModel(ollamaModels, "code") : selectedProviderStatus?.defaultModel ?? null),
    fast: modelConfig.fast || (selectedProvider === "ollama" ? preferredOllamaModel(ollamaModels, "fast") : selectedProviderStatus?.defaultModel ?? null),
    vision: modelConfig.vision || (selectedProvider === "ollama" ? preferredOllamaModel(ollamaModels, "vision") : selectedProviderStatus?.defaultModel ?? null),
  };

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
          activeTool={activeTool}
          setActiveTool={setActiveTool}
          canUndo={undoDepth > 0}
          canRedo={redoDepth > 0}
          onUndo={undoTransform}
          onRedo={redoTransform}
          onOpenSettings={() => { setSettingsTab("project"); setSettingsOpen(true); }}
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
                projectPath={projectPath}
              />
            )}
            {leftTab === "files" && (
              <FileBrowser
                projectPath={projectPath}
                onOpenScript={handleOpenScript}
                onOpenScene={handleOpenScene}
                onFilesChange={setProjectFiles}
                onSceneChange={handleSceneChange}
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
              tilemapEdit={tilemapEdit}
              onTilemapEditChange={setTilemapEdit}
              onSceneChange={handleSceneChange}
              projectPath={projectPath}
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
                key={pendingProposal.changes.map(c => c.id).join(",")}
                proposal={pendingProposal}
                onSceneChange={handleSceneChange}
                onClose={() => setPendingProposal(null)}
                onAllResolved={() => {
                  const key = pendingProposal.changes.map(c => c.id).join(",");
                  setResolvedProposalKeys(prev => new Set([...prev, key]));
                }}
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
                tilemapEdit={tilemapEdit}
                onTilemapEditChange={setTilemapEdit}
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
          resolvedProposalKeys={resolvedProposalKeys}
        />
      )}

      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        initialTab={settingsTab}
        projectPath={projectPath}
        engineReady={engineReady}
        provider={selectedProvider}
        setProvider={p => {
          setSelectedProvider(p);
          const status = providerStatuses.find(s => s.provider === p);
          setModelConfig(p === "ollama"
            ? defaultModelConfig(ollamaModels)
            : { assistant: status?.defaultModel ?? "", code: status?.defaultModel ?? "", fast: status?.defaultModel ?? "", vision: status?.defaultModel ?? "" });
        }}
        modelConfig={modelConfig}
        setModelForRole={(role, model) => setModelConfig(current => ({ ...current, [role]: model }))}
        ollamaModels={ollamaModels}
        providerStatuses={providerStatuses}
        onRefreshProviders={refreshProviderStatuses}
        aiModelRoles={AI_MODEL_ROLES}
        preferredOllamaModel={preferredOllamaModel}
      />

      <style>{`
        @keyframes v3dot {
          0%, 100% { opacity: 0.25; transform: translateY(0); }
          50% { opacity: 1; transform: translateY(-3px); }
        }
      `}</style>
    </ContextMenuProvider>
  );
}

