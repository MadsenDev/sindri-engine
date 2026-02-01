import { useCallback, useEffect, useRef, useState } from "react";
import type { ProjectInfo, SceneTab } from "../types";
import { getSceneName } from "../config";

interface UseSceneTabsOptions {
  project: ProjectInfo | null;
  sceneDirty: boolean;
}

export default function useSceneTabs({ project, sceneDirty }: UseSceneTabsOptions) {
  const [sceneTabs, setSceneTabs] = useState<SceneTab[]>([]);
  const [activeSceneId, setActiveSceneId] = useState<string | null>(null);
  const sceneTabCounter = useRef(1);

  const createUntitledTab = useCallback((): SceneTab => {
    const index = sceneTabCounter.current++;
    const name = `Untitled ${index}`;
    return {
      id: `untitled-${index}`,
      name,
      path: null,
      isDirty: false,
    };
  }, []);

  const setActiveTabDirty = useCallback(
    (dirty: boolean) => {
      setSceneTabs((prev) =>
        prev.map((tab) =>
          tab.id === activeSceneId ? { ...tab, isDirty: dirty } : tab
        )
      );
    },
    [activeSceneId]
  );

  const applySceneSavePath = useCallback(
    (path: string) => {
      setSceneTabs((prev) => {
        const active = prev.find((tab) => tab.id === activeSceneId);
        if (!active) {
          return prev;
        }
        const name = getSceneName(path, active.name);
        const filtered = prev.filter(
          (tab) => tab.id === active.id || tab.path !== path
        );
        return filtered.map((tab) =>
          tab.id === active.id ? { ...tab, name, path, isDirty: false } : tab
        );
      });
    },
    [activeSceneId]
  );

  const upsertSceneTabForPath = useCallback((path: string) => {
    setSceneTabs((prev) => {
      const existing = prev.find((tab) => tab.path === path);
      const name = getSceneName(path, "Scene");
      if (existing) {
        setActiveSceneId(existing.id);
        return prev.map((tab) =>
          tab.id === existing.id ? { ...tab, name, path } : tab
        );
      }
      const id = `scene-${sceneTabCounter.current++}`;
      setActiveSceneId(id);
      return [...prev, { id, name, path, isDirty: false }];
    });
  }, []);

  const ensureSceneTabs = useCallback(() => {
    setSceneTabs((prev) => {
      if (prev.length > 0) {
        return prev;
      }
      const tab = createUntitledTab();
      setActiveSceneId(tab.id);
      return [tab];
    });
  }, [createUntitledTab]);

  useEffect(() => {
    if (!project) {
      setSceneTabs([]);
      setActiveSceneId(null);
      return;
    }
    ensureSceneTabs();
  }, [project, ensureSceneTabs]);

  useEffect(() => {
    if (!activeSceneId) {
      return;
    }
    setSceneTabs((prev) =>
      prev.map((tab) =>
        tab.id === activeSceneId ? { ...tab, isDirty: sceneDirty } : tab
      )
    );
  }, [activeSceneId, sceneDirty]);

  return {
    sceneTabs,
    setSceneTabs,
    activeSceneId,
    setActiveSceneId,
    createUntitledTab,
    applySceneSavePath,
    upsertSceneTabForPath,
    setActiveTabDirty,
  };
}
