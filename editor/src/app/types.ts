export interface EntityInfo {
  id: number;
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

export type PendingSceneAction =
  | "closeProject"
  | "loadScene"
  | "newScene"
  | "switchScene"
  | "closeScene";

export interface SceneTab {
  id: string;
  name: string;
  path: string | null;
  isDirty: boolean;
}
