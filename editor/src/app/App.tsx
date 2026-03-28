import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
} from "react";
import * as FlexLayout from "flexlayout-react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import ProjectPanel from "../panels/ProjectPanel";
import Toolbar, { Tool } from "../components/Toolbar";
import Welcome from "../components/Welcome";
import ScenePanel from "../panels/ScenePanel";
import PlayPanel from "../panels/PlayPanel";
import HierarchyPanel from "../panels/HierarchyPanel";
import InspectorPanel from "../panels/InspectorPanel";
import ConsolePanel from "../panels/ConsolePanel";
import type { ViewportHandle } from "../components/Viewport";
import MenuBar from "./ui/MenuBar";
import SceneTabs from "./ui/SceneTabs";
import ConfirmDialog from "./ui/ConfirmDialog";
import SceneContextMenu from "./ui/SceneContextMenu";
import useSceneTabs from "./hooks/useSceneTabs";
import { defaultLayout, loadLayout, layoutStorageKey } from "./layout";
import { panelDefinitions, presetOptions } from "./config";
import type {
  CameraInfo,
  ConsoleEntry,
  EntityInfo,
  PendingSceneAction,
  PlayState,
  ProjectInfo,
} from "./types";
import "./App.css";
import "flexlayout-react/style/dark.css";

export default function App() {
  const appWindow = getCurrentWindow();
  const [layoutModel, setLayoutModel] = useState(() =>
    FlexLayout.Model.fromJson(loadLayout())
  );
  const layoutModelRef = useRef(layoutModel);
  const viewportRef = useRef<ViewportHandle>(null);
  const worldRevisionRef = useRef<number | null>(null);
  const playCameraRef = useRef<CameraInfo | null>(null);

  const [project, setProject] = useState<ProjectInfo | null>(null);
  const [entities, setEntities] = useState<EntityInfo[]>([]);
  const [selectedEntityId, setSelectedEntityId] = useState<number | null>(null);
  const [selectedEntityIds, setSelectedEntityIds] = useState<number[]>([]);
  const [tool, setTool] = useState<Tool>("move");
  const [playState, setPlayState] = useState<PlayState>("stopped");
  const [playRenderTick, setPlayRenderTick] = useState(0);
  const [playCamera, setPlayCamera] = useState<CameraInfo | null>(null);
  const [sceneDirty, setSceneDirty] = useState(false);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);
  const [refreshToken, setRefreshToken] = useState(0);
  const [inspectorRefresh, setInspectorRefresh] = useState(0);
  const [gridSize, setGridSize] = useState(50);
  const [createMenuOpen, setCreateMenuOpen] = useState(false);
  const [panelMenuOpen, setPanelMenuOpen] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [consoleEntries, setConsoleEntries] = useState<ConsoleEntry[]>([]);
  const [pendingSceneAction, setPendingSceneAction] =
    useState<PendingSceneAction | null>(null);
  const [pendingSceneTargetId, setPendingSceneTargetId] = useState<string | null>(null);
  const {
    sceneTabs,
    setSceneTabs,
    activeSceneId,
    setActiveSceneId,
    createUntitledTab,
    applySceneSavePath,
    upsertSceneTabForPath,
    setActiveTabDirty,
  } = useSceneTabs({ project, sceneDirty });
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    world?: { x: number; y: number };
  } | null>(null);

  const handleMinimize = useCallback(async () => {
    try {
      await appWindow.minimize();
    } catch (e) {
      console.error("Window minimize failed:", e);
    }
  }, [appWindow]);

  const handleToggleMaximize = useCallback(async () => {
    try {
      await appWindow.toggleMaximize();
    } catch (e) {
      console.error("Window toggle maximize failed:", e);
    }
  }, [appWindow]);

  const handleCloseWindow = useCallback(async () => {
    try {
      await appWindow.close();
    } catch (e) {
      console.error("Window close failed:", e);
    }
  }, [appWindow]);

  const handleStartDragging = useCallback(
    async (event: ReactMouseEvent) => {
      if (event.button !== 0) {
        return;
      }
      try {
        await appWindow.startDragging();
      } catch (e) {
        console.error("Window drag failed:", e);
      }
    },
    [appWindow]
  );

  useEffect(() => {
    layoutModelRef.current = layoutModel;
  }, [layoutModel]);

  useEffect(() => {
    playCameraRef.current = playCamera;
  }, [playCamera]);

  const pushStatus = useCallback((message: string) => {
    setStatusMessage(message);
    setConsoleEntries((prev): ConsoleEntry[] => [
      {
        id: Date.now() + prev.length,
        level: "info" as const,
        message,
        timestamp: new Date().toLocaleTimeString(),
      },
      ...prev,
    ].slice(0, 200));
    window.setTimeout(() => setStatusMessage(null), 3000);
  }, []);

  const pushError = useCallback((message: string) => {
    setStatusMessage(message);
    setConsoleEntries((prev): ConsoleEntry[] => [
      {
        id: Date.now() + prev.length,
        level: "error" as const,
        message,
        timestamp: new Date().toLocaleTimeString(),
      },
      ...prev,
    ].slice(0, 200));
    window.setTimeout(() => setStatusMessage(null), 5000);
  }, []);

  const refreshEditorState = useCallback(async () => {
    const [undoable, redoable, dirty] = await Promise.all([
      invoke<boolean>("can_undo"),
      invoke<boolean>("can_redo"),
      invoke<boolean>("scene_is_dirty"),
    ]);
    setCanUndo(undoable);
    setCanRedo(redoable);
    setSceneDirty(dirty);
  }, []);

  const refreshEntities = useCallback(async (force = false) => {
    const revision = await invoke<number>("world_revision_get");
    if (!force && worldRevisionRef.current === revision) {
      return;
    }
    worldRevisionRef.current = revision;
    const data = await invoke<EntityInfo[]>("entities_list");
    setEntities(data);
    setSelectedEntityIds((prev) => {
      const existing = new Set(data.map((entity) => entity.id));
      const next = prev.filter((id) => existing.has(id));
      return next;
    });
    setSelectedEntityId((prev) => {
      if (prev !== null && data.some((entity) => entity.id === prev)) {
        return prev;
      }
      return null;
    });
  }, []);

  const loadProjectInfo = useCallback(async () => {
    const current = await invoke<ProjectInfo | null>("project_get_current");
    setProject(current ?? null);
    return current ?? null;
  }, []);

  useEffect(() => {
    const bootstrap = async () => {
      const current = await loadProjectInfo();
      const state = await invoke<PlayState>("play_state_get");
      setPlayState(state);
      await refreshEditorState();
      if (current) {
        await refreshEntities(true);
      }
    };
    bootstrap();
  }, [loadProjectInfo, refreshEditorState, refreshEntities]);

  useEffect(() => {
    viewportRef.current?.setGridSize(gridSize);
  }, [gridSize]);

  useEffect(() => {
    if (playState === "stopped") {
      setPlayCamera(null);
      return;
    }

    let frameId = 0;
    let active = true;
    let lastTime = 0;
    let accumulator = 0;
    let entityRefreshAccumulator = 0;
    let cameraRefreshAccumulator = 0;
    let inFlight = false;
    const targetFps = playState === "playing" ? 24 : 12;
    const frameInterval = 1000 / targetFps;

    const tick = async (now: number) => {
      if (!active) return;
      if (lastTime === 0) {
        lastTime = now;
      }
      const delta = now - lastTime;
      lastTime = now;
      accumulator += delta;
      entityRefreshAccumulator += delta;
      cameraRefreshAccumulator += delta;

      if (!inFlight && accumulator >= frameInterval) {
        inFlight = true;
        accumulator = 0;
        try {
          if (playState === "playing") {
            await invoke("play_step_physics", { dt: 1 / 60 });
          }
          if (cameraRefreshAccumulator >= 250 || playCameraRef.current === null) {
            const nextCamera = await invoke<CameraInfo | null>("active_camera_info");
            if (active) {
              setPlayCamera(nextCamera);
            }
            cameraRefreshAccumulator = 0;
          }
          if (entityRefreshAccumulator >= 1500) {
            await refreshEntities();
            entityRefreshAccumulator = 0;
          }
          if (active) {
            setPlayRenderTick((prev) => prev + 1);
          }
        } catch (e) {
          console.error("Shared play scheduler failed:", e);
        } finally {
          inFlight = false;
        }
      }

      frameId = requestAnimationFrame(tick);
    };

    frameId = requestAnimationFrame(tick);
    return () => {
      active = false;
      cancelAnimationFrame(frameId);
    };
  }, [playState, refreshEntities]);

  useEffect(() => {
    if (!createMenuOpen && !contextMenu && !panelMenuOpen) {
      return;
    }
    const closeMenus = () => {
      setCreateMenuOpen(false);
      setPanelMenuOpen(false);
      setContextMenu(null);
    };
    window.addEventListener("click", closeMenus);
    return () => window.removeEventListener("click", closeMenus);
  }, [createMenuOpen, contextMenu, panelMenuOpen]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const isTyping =
        target?.tagName === "INPUT" ||
        target?.tagName === "TEXTAREA" ||
        target?.getAttribute("contenteditable") === "true";
      if (isTyping) return;

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        handleSave();
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
        event.preventDefault();
        handleUndo();
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "y") {
        event.preventDefault();
        handleRedo();
        return;
      }
      if (event.key === "Delete" || event.key === "Backspace") {
        handleDelete();
        return;
      }
      if (event.key.toLowerCase() === "w") {
        setTool("move");
      } else if (event.key.toLowerCase() === "e") {
        setTool("rotate");
      } else if (event.key.toLowerCase() === "r") {
        setTool("scale");
      } else if (event.key === "Escape") {
        updateSelection([]);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  const updateSelection = (ids: number[]) => {
    setSelectedEntityIds(ids);
    setSelectedEntityId(ids.length > 0 ? ids[ids.length - 1] : null);
    invoke("selection_set", { ids }).catch(() => undefined);
    setInspectorRefresh((prev) => prev + 1);
  };

  const handleEntityClick = (entityId: number) => {
    updateSelection([entityId]);
  };

  const runPendingAction = useCallback(
    async (
      action: PendingSceneAction,
      forceCloseProject: boolean,
      targetIdOverride?: string | null
    ) => {
      const targetId = targetIdOverride ?? pendingSceneTargetId;
      if (action === "closeProject") {
        try {
          await invoke("project_close", { force: forceCloseProject });
          setProject(null);
          updateSelection([]);
          setEntities([]);
          pushStatus("Project closed");
          setSceneTabs([]);
          setActiveSceneId(null);
        } catch (e) {
          console.error("Close project failed:", e);
          pushStatus("Close project failed");
        }
        return;
      }

      if (action === "newScene") {
        try {
          await invoke("scene_new");
          const tab = createUntitledTab();
          setSceneTabs((prev) => [...prev, tab]);
          setActiveSceneId(tab.id);
          updateSelection([]);
          await refreshEntities();
          await refreshEditorState();
        } catch (e) {
          console.error("New scene failed:", e);
          pushStatus("New scene failed");
        }
        return;
      }

      if (action === "loadScene") {
        try {
          const path = await open({
            filters: [
              {
                name: "Scene",
                extensions: ["json"],
              },
            ],
          });
          if (!path || typeof path !== "string") {
            return;
          }
          await invoke("scene_load", { path });
          upsertSceneTabForPath(path);
          updateSelection([]);
          await refreshEntities();
          await refreshEditorState();
          setRefreshToken((prev) => prev + 1);
          pushStatus("Scene loaded");
        } catch (e) {
          console.error("Load failed:", e);
          pushStatus("Load failed");
        }
        return;
      }

      if (action === "switchScene") {
        if (!targetId) {
          return;
        }
        const target = sceneTabs.find((tab) => tab.id === targetId);
        if (!target || target.id === activeSceneId) {
          return;
        }
        try {
          if (target.path) {
            await invoke("scene_load", { path: target.path });
          } else {
            await invoke("scene_new");
          }
          setActiveSceneId(target.id);
          updateSelection([]);
          await refreshEntities();
          await refreshEditorState();
          setRefreshToken((prev) => prev + 1);
        } catch (e) {
          console.error("Switch scene failed:", e);
          pushStatus("Switch scene failed");
        }
        return;
      }

      if (action === "closeScene") {
        if (!targetId) {
          return;
        }
        setSceneTabs((prev) => prev.filter((tab) => tab.id !== targetId));
        if (activeSceneId === targetId) {
          const remaining = sceneTabs.filter((tab) => tab.id !== targetId);
          if (remaining.length > 0) {
            await runPendingAction("switchScene", forceCloseProject, remaining[0].id);
          } else {
            await invoke("scene_new");
            const tab = createUntitledTab();
            setSceneTabs([tab]);
            setActiveSceneId(tab.id);
            updateSelection([]);
            await refreshEntities();
            await refreshEditorState();
          }
        }
      }
    },
    [
      pushStatus,
      refreshEditorState,
      refreshEntities,
      updateSelection,
      setProject,
      setEntities,
      open,
      createUntitledTab,
      upsertSceneTabForPath,
      pendingSceneTargetId,
      sceneTabs,
      activeSceneId,
    ]
  );

  const requestSceneAction = useCallback(
    async (action: PendingSceneAction, targetId?: string | null) => {
      setPendingSceneTargetId(targetId ?? null);
      if (!sceneDirty) {
        await runPendingAction(action, false, targetId ?? null);
        setPendingSceneTargetId(null);
        return;
      }
      setPendingSceneAction(action);
    },
    [runPendingAction, sceneDirty]
  );

  const handleConfirmCancel = () => {
    setPendingSceneAction(null);
    setPendingSceneTargetId(null);
  };

  const handleConfirmDiscard = async () => {
    const action = pendingSceneAction;
    setPendingSceneAction(null);
    if (!action) return;
    if (action === "switchScene" || action === "closeScene") {
      setActiveTabDirty(false);
    }
    await runPendingAction(action, true);
    setPendingSceneTargetId(null);
  };

  const handleConfirmSave = async () => {
    const action = pendingSceneAction;
    if (!action) {
      return;
    }
    try {
      const savedPath = await invoke<string>("scene_save", { path: null });
      applySceneSavePath(savedPath);
      await refreshEditorState();
      setRefreshToken((prev) => prev + 1);
      pushStatus("Scene saved");
      setPendingSceneAction(null);
      await runPendingAction(action, false);
      setPendingSceneTargetId(null);
    } catch (e) {
      console.error("Save failed:", e);
      pushStatus("Save failed");
    }
  };

  const handleCreateEntity = async (
    preset: string,
    world?: { x: number; y: number }
  ) => {
    try {
      const position = world ? [world.x, world.y] : undefined;
      const entityId = await invoke<number>("entity_create_preset", {
        preset,
        position,
      });
      await refreshEntities();
      updateSelection([entityId]);
      await refreshEditorState();
      setCreateMenuOpen(false);
      setContextMenu(null);
    } catch (e) {
      console.error("Create entity failed:", e);
      pushStatus("Failed to create entity");
    }
  };

  const handleDelete = async () => {
    const ids = selectedEntityIds.length > 0 ? selectedEntityIds : selectedEntityId ? [selectedEntityId] : [];
    if (ids.length === 0) {
      return;
    }
    try {
      for (const id of ids) {
        await invoke("entity_delete", { entityId: id });
      }
      updateSelection([]);
      await refreshEntities();
      await refreshEditorState();
    } catch (e) {
      console.error("Delete entity failed:", e);
      pushStatus("Delete failed");
    }
  };

  const handleDuplicate = async () => {
    if (selectedEntityId === null) {
      return;
    }
    try {
      const newId = await invoke<number>("entity_duplicate", {
        entityId: selectedEntityId,
      });
      await refreshEntities();
      updateSelection([newId]);
      await refreshEditorState();
    } catch (e) {
      console.error("Duplicate failed:", e);
      pushStatus("Duplicate failed");
    }
  };

  const handleUndo = async () => {
    try {
      await invoke("undo");
      await refreshEntities();
      await refreshEditorState();
    } catch (e) {
      console.error("Undo failed:", e);
      pushStatus("Undo failed");
    }
  };

  const handleRedo = async () => {
    try {
      await invoke("redo");
      await refreshEntities();
      await refreshEditorState();
    } catch (e) {
      console.error("Redo failed:", e);
      pushStatus("Redo failed");
    }
  };

  const handleNewScene = async () => {
    await requestSceneAction("newScene");
  };

  const handleSave = async () => {
    try {
      const savedPath = await invoke<string>("scene_save", { path: null });
      applySceneSavePath(savedPath);
      await refreshEditorState();
      setRefreshToken((prev) => prev + 1);
      pushStatus("Scene saved");
    } catch (e) {
      console.error("Save failed:", e);
      pushStatus("Save failed");
    }
  };

  const handleSaveAs = async () => {
    try {
      const path = await save({
        filters: [
          {
            name: "Scene",
            extensions: ["json"],
          },
        ],
      });
      if (!path || typeof path !== "string") {
        return;
      }
      const savedPath = await invoke<string>("scene_save", { path });
      applySceneSavePath(savedPath);
      await refreshEditorState();
      setRefreshToken((prev) => prev + 1);
      pushStatus("Scene saved");
    } catch (e) {
      console.error("Save as failed:", e);
      pushStatus("Save failed");
    }
  };

  const handleLoad = async () => {
    await requestSceneAction("loadScene");
  };

  const handleOpenScenePath = async (path: string) => {
    try {
      await invoke("scene_load", { path });
      upsertSceneTabForPath(path);
      updateSelection([]);
      await refreshEntities();
      await refreshEditorState();
      setRefreshToken((prev) => prev + 1);
      pushStatus(`Opened ${path.split(/[/\\\\]/).pop()}`);
    } catch (e) {
      console.error("Open scene failed:", e);
      pushError("Open scene failed");
    }
  };

  const handlePlay = async () => {
    try {
      await invoke("play_start");
      setPlayState("playing");
      await refreshEditorState();
    } catch (e) {
      console.error("Play failed:", e);
      pushError("Play failed");
    }
  };

  const handlePause = async () => {
    try {
      await invoke("play_pause");
      setPlayState("paused");
    } catch (e) {
      console.error("Pause failed:", e);
      pushError("Pause failed");
    }
  };

  const handleResume = async () => {
    try {
      await invoke("play_resume");
      setPlayState("playing");
    } catch (e) {
      console.error("Resume failed:", e);
      pushError("Resume failed");
    }
  };

  const handleStepFrame = async () => {
    try {
      await invoke("play_step_frame", { dt: 1 / 60 });
      await refreshEntities();
    } catch (e) {
      console.error("Step frame failed:", e);
      pushError("Step failed");
    }
  };

  const handleStop = async () => {
    try {
      await invoke("play_stop");
      setPlayState("stopped");
      await refreshEntities();
      await refreshEditorState();
    } catch (e) {
      console.error("Stop failed:", e);
      pushError("Stop failed");
    }
  };

  const handleCloseProject = async () => {
    await requestSceneAction("closeProject");
  };

  const handleProjectOpen = async () => {
    const current = await loadProjectInfo();
    if (current) {
      await refreshEntities();
      await refreshEditorState();
      setRefreshToken((prev) => prev + 1);
    }
  };

  const handleImportTexture = async () => {
    try {
      const filePath = await open({
        filters: [
          {
            name: "Image",
            extensions: ["png", "jpg", "jpeg", "gif", "webp"],
          },
        ],
      });
      if (!filePath || typeof filePath !== "string") {
        return;
      }
      await invoke("asset_import_texture", { path: filePath });
      setRefreshToken((prev) => prev + 1);
      pushStatus("Texture imported");
    } catch (e) {
      console.error("Import texture failed:", e);
      pushStatus("Import failed");
    }
  };

  const handleSavePrefab = async () => {
    if (selectedEntityId === null) {
      pushStatus("Select an entity to save prefab");
      return;
    }
    try {
      const path = await save({
        filters: [
          {
            name: "Prefab",
            extensions: ["prefab", "json"],
          },
        ],
      });
      if (!path || typeof path !== "string") {
        return;
      }
      const savedPath = await invoke<string>("prefab_save", {
        entityId: selectedEntityId,
        path,
      });
      setRefreshToken((prev) => prev + 1);
      pushStatus(`Prefab saved: ${savedPath.split(/[/\\\\]/).pop()}`);
    } catch (e) {
      console.error("Save prefab failed:", e);
      pushStatus("Prefab save failed");
    }
  };

  const handleInstantiatePrefab = async () => {
    try {
      const path = await open({
        filters: [
          {
            name: "Prefab",
            extensions: ["prefab", "json"],
          },
        ],
      });
      if (!path || typeof path !== "string") {
        return;
      }
      const newRoots = await invoke<number[]>("prefab_instantiate", { path });
      if (newRoots.length > 0) {
        updateSelection(newRoots);
      }
      await refreshEntities();
      await refreshEditorState();
      setRefreshToken((prev) => prev + 1);
      pushStatus("Prefab instantiated");
    } catch (e) {
      console.error("Instantiate prefab failed:", e);
      pushStatus("Prefab instantiate failed");
    }
  };

  const handleInstantiatePrefabPath = async (
    path: string,
    world?: { x: number; y: number }
  ) => {
    try {
      const command = world ? "prefab_instantiate_at" : "prefab_instantiate";
      const payload = world
        ? { path, position: [world.x, world.y] }
        : { path };
      const newRoots = await invoke<number[]>(command, payload);
      if (newRoots.length > 0) {
        updateSelection(newRoots);
      }
      await refreshEntities();
      await refreshEditorState();
      setRefreshToken((prev) => prev + 1);
      pushStatus(`Prefab added: ${path.split(/[/\\\\]/).pop()}`);
    } catch (e) {
      console.error("Instantiate prefab failed:", e);
      pushError("Prefab instantiate failed");
    }
  };

  const handleReparent = async (entityId: number, parentId: number | null) => {
    try {
      await invoke("entity_reparent", { entityId, parentId });
      await refreshEntities();
      await refreshEditorState();
    } catch (e) {
      console.error("Reparent failed:", e);
      pushError("Reparent failed");
    }
  };

  const handleViewportAssetDrop = async (
    asset: { path: string; kind: string },
    world: { x: number; y: number }
  ) => {
    try {
      if (asset.kind === "prefab") {
        await handleInstantiatePrefabPath(asset.path, world);
        return;
      }

      if (asset.kind === "texture") {
        const targetEntity =
          selectedEntityId !== null &&
          entities.find((entity) => entity.id === selectedEntityId)?.has_sprite
            ? selectedEntityId
            : null;

        if (targetEntity !== null) {
          await invoke("sprite_set_texture_path", {
            entityId: targetEntity,
            path: asset.path,
          });
          setInspectorRefresh((prev) => prev + 1);
          await refreshEditorState();
          pushStatus("Texture assigned");
          return;
        }

        const entityId = await invoke<number>("entity_create_preset", {
          preset: "sprite",
          position: [world.x, world.y],
        });
        await invoke("sprite_set_texture_path", {
          entityId,
          path: asset.path,
        });
        await refreshEntities();
        updateSelection([entityId]);
        await refreshEditorState();
        pushStatus(`Sprite created from ${asset.path.split(/[/\\\\]/).pop()}`);
      }
    } catch (e) {
      console.error("Asset drop failed:", e);
      pushError("Asset drop failed");
    }
  };

  const handleSceneTabSelect = async (tabId: string) => {
    if (tabId === activeSceneId) {
      return;
    }
    await requestSceneAction("switchScene", tabId);
  };

  const handleSceneTabClose = async (tabId: string) => {
    if (tabId === activeSceneId) {
      await requestSceneAction("closeScene", tabId);
      return;
    }
    setSceneTabs((prev) => prev.filter((tab) => tab.id !== tabId));
  };

  const handleContextMenuOpen = (
    screen: { x: number; y: number },
    world?: { x: number; y: number }
  ) => {
    setCreateMenuOpen(false);
    setContextMenu({ x: screen.x, y: screen.y, world });
  };

  const openPanel = (panelId: string) => {
    const model = layoutModelRef.current;
    const panel = panelDefinitions.find((entry) => entry.id === panelId);
    if (!panel) return;
    setPanelMenuOpen(false);

    const existing = model.getNodeById(panelId);
    if (existing) {
      model.doAction(FlexLayout.Actions.selectTab(panelId));
      return;
    }

    const target =
      model.getActiveTabset() ?? model.getFirstTabSet(model.getRoot());
    if (!target) {
      const singleLayout: FlexLayout.IJsonModel = {
        global: defaultLayout.global,
        layout: {
          type: "row",
          children: [
            {
              type: "tabset",
              id: "main-tabset",
              children: [
                {
                  type: "tab",
                  id: panel.id,
                  name: panel.name,
                  component: panel.component,
                },
              ],
            },
          ],
        },
      };
      setLayoutModel(FlexLayout.Model.fromJson(singleLayout));
      return;
    }

    model.doAction(
      FlexLayout.Actions.addNode(
        {
          type: "tab",
          id: panel.id,
          name: panel.name,
          component: panel.component,
        },
        target.getId(),
        FlexLayout.DockLocation.CENTER,
        -1,
        true
      )
    );
  };

  const handleResetLayout = () => {
    setLayoutModel(FlexLayout.Model.fromJson(defaultLayout));
    setPanelMenuOpen(false);
    try {
      window.localStorage.removeItem(layoutStorageKey);
    } catch {
      // Ignore storage errors.
    }
  };

  const handleModelChange = useCallback((model: FlexLayout.Model) => {
    try {
      window.localStorage.setItem(layoutStorageKey, JSON.stringify(model.toJson()));
    } catch {
      // Ignore storage errors.
    }
  }, []);

  const factory = (node: FlexLayout.TabNode) => {
    const component = node.getComponent();
    switch (component) {
      case "scene":
        return (
          <ScenePanel
            createMenuOpen={createMenuOpen}
            onToggleCreateMenu={() => setCreateMenuOpen((prev) => !prev)}
            presetOptions={presetOptions}
            onCreateEntity={handleCreateEntity}
            onFrameSelection={() => viewportRef.current?.frameSelection()}
            onResetCamera={() => viewportRef.current?.resetCamera()}
            selectedEntityIds={selectedEntityIds}
            selectedEntityId={selectedEntityId}
            gridSize={gridSize}
            onGridSizeChange={(value) => setGridSize(value)}
            viewportRef={viewportRef}
            entities={entities}
            onEntityClick={handleEntityClick}
            onSelectionChange={updateSelection}
            onTransformChange={refreshEntities}
            onContextMenuOpen={handleContextMenuOpen}
            onAssetDrop={handleViewportAssetDrop}
            isPlaying={playState !== "stopped"}
            tool={tool}
          />
        );
      case "play":
        return (
          <PlayPanel
            playState={playState}
            renderTick={playRenderTick}
            camera={playCamera}
          />
        );
      case "hierarchy":
        return (
          <HierarchyPanel
            entities={entities}
            selectedEntityId={selectedEntityId}
            onDuplicate={handleDuplicate}
            onDelete={handleDelete}
            onSavePrefab={handleSavePrefab}
            onInstantiatePrefab={handleInstantiatePrefab}
            onEntityClick={handleEntityClick}
            onContextMenuOpen={(screen) => handleContextMenuOpen(screen)}
            onReparent={handleReparent}
          />
        );
      case "project":
        return (
          <ProjectPanel
            refreshToken={refreshToken}
            onImportTexture={handleImportTexture}
            onRefresh={() => setRefreshToken((prev) => prev + 1)}
            onOpenScene={handleOpenScenePath}
            onInstantiatePrefab={(path) => void handleInstantiatePrefabPath(path)}
          />
        );
      case "inspector":
        return (
          <InspectorPanel
            selectedEntityId={selectedEntityId}
            inspectorRefresh={inspectorRefresh}
          />
        );
      case "console":
        return <ConsolePanel statusMessage={statusMessage} entries={consoleEntries} />;
      default:
        return <div className="panel-body">Unknown panel</div>;
    }
  };

  const hasProject = project !== null;

  return (
    <div className="unity-shell">
      <MenuBar
        hasProject={hasProject}
        projectName={project?.name ?? null}
        sceneDirty={sceneDirty}
        statusMessage={statusMessage}
        panelMenuOpen={panelMenuOpen}
        panelDefinitions={panelDefinitions}
        onTogglePanelMenu={() => setPanelMenuOpen((prev) => !prev)}
        onOpenPanel={openPanel}
        onResetLayout={handleResetLayout}
        onNewScene={handleNewScene}
        onSave={handleSave}
        onSaveAs={handleSaveAs}
        onLoad={handleLoad}
        onCloseProject={handleCloseProject}
        onMinimize={handleMinimize}
        onToggleMaximize={handleToggleMaximize}
        onCloseWindow={handleCloseWindow}
        onStartDragging={handleStartDragging}
      />

      {hasProject && (
        <SceneTabs
          tabs={sceneTabs}
          activeTabId={activeSceneId}
          onSelect={handleSceneTabSelect}
          onClose={handleSceneTabClose}
          onAdd={handleNewScene}
        />
      )}

      {hasProject ? (
        <>
          <div className="unity-toolbar-row">
            <Toolbar
              currentTool={tool}
              onToolChange={setTool}
              playState={playState}
              onUndo={handleUndo}
              onRedo={handleRedo}
              canUndo={canUndo}
              canRedo={canRedo}
              onNewScene={handleNewScene}
              onSave={handleSave}
              onLoad={handleLoad}
              onPlay={handlePlay}
              onPause={handlePause}
              onResume={handleResume}
              onStep={handleStepFrame}
              onStop={handleStop}
            />
          </div>

          <div className="editor-dock">
            <FlexLayout.Layout
              model={layoutModel}
              factory={factory}
              onModelChange={handleModelChange}
            />
          </div>

          {contextMenu && (
            <SceneContextMenu
              x={contextMenu.x}
              y={contextMenu.y}
              world={contextMenu.world}
              presetOptions={presetOptions}
              showSelectionActions={selectedEntityId !== null || selectedEntityIds.length > 0}
              onCreateEntity={handleCreateEntity}
              onDuplicate={handleDuplicate}
              onDelete={handleDelete}
            />
          )}
        </>
      ) : (
        <div className="welcome-shell">
          <Welcome onProjectOpen={handleProjectOpen} />
        </div>
      )}

      <ConfirmDialog
        isOpen={Boolean(pendingSceneAction)}
        onSave={handleConfirmSave}
        onDiscard={handleConfirmDiscard}
        onCancel={handleConfirmCancel}
      />
    </div>
  );
}
