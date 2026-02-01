import * as FlexLayout from "flexlayout-react";

export const layoutStorageKey = "forge2d.editor.layout.v5";

export const defaultLayout: FlexLayout.IJsonModel = {
  global: {
    tabEnableClose: true,
    tabSetEnableClose: false,
    tabSetEnableMaximize: true,
    tabSetEnableDivide: true,
  },
  layout: {
    type: "row",
    weight: 100,
    children: [
      {
        type: "column",
        id: "left-column",
        weight: 60,
        children: [
          {
            type: "tabset",
            id: "scene-tabset",
            weight: 65,
            children: [
              { type: "tab", id: "scene", name: "Scene", component: "scene" },
              { type: "tab", id: "console", name: "Console", component: "console" },
            ],
          },
          {
            type: "tabset",
            id: "play-tabset",
            weight: 35,
            children: [
              { type: "tab", id: "play", name: "Play", component: "play" },
            ],
          },
        ],
      },
      {
        type: "tabset",
        id: "hierarchy-tabset",
        weight: 14,
        children: [
          { type: "tab", id: "hierarchy", name: "Hierarchy", component: "hierarchy" },
        ],
      },
      {
        type: "tabset",
        id: "project-tabset",
        weight: 13,
        children: [
          { type: "tab", id: "project", name: "Project", component: "project" },
        ],
      },
      {
        type: "tabset",
        id: "inspector-tabset",
        weight: 13,
        children: [
          { type: "tab", id: "inspector", name: "Inspector", component: "inspector" },
        ],
      },
    ],
  },
};

export const loadLayout = () => {
  if (typeof window === "undefined") {
    return defaultLayout;
  }
  try {
    const raw = window.localStorage.getItem(layoutStorageKey);
    if (!raw) {
      return defaultLayout;
    }
    const json = JSON.parse(raw);
    return json;
  } catch {
    return defaultLayout;
  }
};
