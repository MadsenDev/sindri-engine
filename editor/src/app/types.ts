export interface EntityInfo {
  id: number;
  name: string;
  has_transform: boolean;
  has_sprite: boolean;
  has_physics: boolean;
  has_camera: boolean;
  parent_id: number | null;
  children: number[];
}

export interface ProjectInfo {
  name: string;
  path: string;
  version: string;
}

export interface PanelDefinition {
  id: string;
  name: string;
  component: string;
}

export type PendingSceneAction = "closeProject" | "loadScene" | "newScene";

export type PlayState = "stopped" | "playing" | "paused";

export interface ConsoleEntry {
  id: number;
  level: "info" | "error";
  message: string;
  timestamp: string;
}

export interface CameraInfo {
  entity_id: number;
  world_position: [number, number];
  rotation: number;
  zoom: number;
  offset: [number, number];
  active: boolean;
}
