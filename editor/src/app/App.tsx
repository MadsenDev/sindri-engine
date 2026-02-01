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
  EntityInfo,
  PendingSceneAction,
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

  const [project, setProject] = useState<ProjectInfo | null>(null);
  const [entities, setEntities] = useState<EntityInfo[]>([]);
  const [selectedEntityId, setSelectedEntityId] = useState<number | null>(null);
  const [selectedEntityIds, setSelectedEntityIds] = useState<number[]>([]);
  const [tool, setTool] = useState<Tool>("move");
  const [isPlaying, setIsPlaying] = useState(false);
  const [sceneDirty, setSceneDirty] = useState(false);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);
  const [refreshToken, setRefreshToken] = useState(0);
  const [inspectorRefresh, setInspectorRefresh] = useState(0);
  const [gridSize, setGridSize] = useState(50);
  const [createMenuOpen, setCreateMenuOpen] = useState(false);
  const [panelMenuOpen, setPanelMenuOpen] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
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

  const pushStatus = useCallback((message: string) => {
    setStatusMessage(message);
    window.setTimeout(() => setStatusMessage(null), 3000);
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

  const refreshEntities = useCallback(async () => {
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
      const playing = await invoke<boolean>("play_is_playing");
      setIsPlaying(playing);
      await refreshEditorState();
      if (current) {
        await refreshEntities();
      }
    };
    bootstrap();
  }, [loadProjectInfo, refreshEditorState, refreshEntities]);

  useEffect(() => {
    viewportRef.current?.setGridSize(gridSize);
  }, [gridSize]);

  useEffect(() => {
    if (!isPlaying) {
      return;
    }
    let inFlight = false;
    const interval = window.setInterval(async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        await invoke("play_step_physics", { dt: 1 / 60 });
      } catch (e) {
        console.error("Physics step failed:", e);
      } finally {
        inFlight = false;
      }
    }, 1000 / 60);

    return () => window.clearInterval(interval);
  }, [isPlaying]);

  useEffect(() => {
    if (!isPlaying) {
      return;
    }
    const interval = window.setInterval(() => {
      refreshEntities().catch(() => undefined);
    }, 500);
    return () => window.clearInterval(interval);
  }, [isPlaying, refreshEntities]);

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

  const handlePlay = async () => {
    try {
      await invoke("play_start");
      setIsPlaying(true);
      await refreshEditorState();
    } catch (e) {
      console.error("Play failed:", e);
      pushStatus("Play failed");
    }
  };

  const handleStop = async () => {
    try {
      await invoke("play_stop");
      setIsPlaying(false);
      await refreshEntities();
      await refreshEditorState();
    } catch (e) {
      console.error("Stop failed:", e);
      pushStatus("Stop failed");
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
            isPlaying={isPlaying}
            tool={tool}
          />
        );
      case "play":
        return <PlayPanel isPlaying={isPlaying} />;
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
          />
        );
      case "project":
        return (
          <ProjectPanel
            refreshToken={refreshToken}
            onImportTexture={handleImportTexture}
            onRefresh={() => setRefreshToken((prev) => prev + 1)}
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
        return <ConsolePanel statusMessage={statusMessage} />;
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
              isPlaying={isPlaying}
              onUndo={handleUndo}
              onRedo={handleRedo}
              canUndo={canUndo}
              canRedo={canRedo}
              onNewScene={handleNewScene}
              onSave={handleSave}
              onLoad={handleLoad}
              onPlay={handlePlay}
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
