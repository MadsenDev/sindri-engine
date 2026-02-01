import type { PanelDefinition } from "./types";

export const panelDefinitions: PanelDefinition[] = [
  { id: "scene", name: "Scene", component: "scene" },
  { id: "play", name: "Play", component: "play" },
  { id: "hierarchy", name: "Hierarchy", component: "hierarchy" },
  { id: "project", name: "Project", component: "project" },
  { id: "inspector", name: "Inspector", component: "inspector" },
  { id: "console", name: "Console", component: "console" },
];

export const presetOptions = [
  { id: "empty", label: "Empty", preset: "empty" },
  { id: "sprite", label: "Sprite", preset: "sprite" },
  { id: "camera", label: "Camera", preset: "camera" },
  { id: "physics", label: "Physics", preset: "physics" },
  { id: "tilemap", label: "Tilemap", preset: "tilemap" },
  { id: "script", label: "Script", preset: "script" },
];

export const getSceneName = (path: string | null, fallback: string) => {
  if (!path) {
    return fallback;
  }
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || fallback;
};
