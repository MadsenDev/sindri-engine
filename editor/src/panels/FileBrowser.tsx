import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useContextMenu } from "../components/ContextMenu";
import AnimClipEditor from "../components/AnimClipEditor";
import type { AnimClip } from "../App";

interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  kind: string; // "dir" | "script" | "scene" | "image" | "audio" | "other"
  children: FileNode[];
}

interface CreateState {
  parentPath: string;
  type: "script" | "scene" | "folder";
  value: string;
}

interface AnimatedSpriteComp {
  texture_path: string;
  cols: number;
  rows: number;
  width: number;
  height: number;
  flip_x: boolean;
  flip_y: boolean;
  tint: [number, number, number, number];
  clips: AnimClip[];
  default_clip: string;
}

interface Props {
  projectPath: string | null;
  onOpenScript: (path: string) => void;
  onOpenScene: (path: string) => void;
  onFilesChange?: (files: { path: string; kind: string; name: string }[]) => void;
  onSceneChange?: () => void;
}

const FILE_ICON: Record<string, string> = {
  script:     "⚡",
  scene:      "◈",
  image:      "▣",
  animclips:  "▶",
  audio:      "♪",
  prefab:     "◆",
  tilepallet: "◧",
  other:      "·",
};

function flattenTree(node: FileNode): { path: string; kind: string; name: string }[] {
  const out: { path: string; kind: string; name: string }[] = [];
  if (!node.is_dir) {
    out.push({ path: node.path, kind: node.kind, name: node.name });
  }
  for (const child of node.children) {
    out.push(...flattenTree(child));
  }
  return out;
}

const inputStyle: React.CSSProperties = {
  flex: 1, height: "18px", background: "var(--bg-3)",
  border: "1px solid var(--accent)", borderRadius: "var(--radius)",
  color: "var(--text-bright)", fontFamily: "var(--font-mono)",
  fontSize: "10px", padding: "0 4px", outline: "none",
};

const toolbarBtnStyle: React.CSSProperties = {
  background: "none", border: "1px solid var(--border)", borderRadius: "var(--radius)",
  color: "var(--text-muted)", fontFamily: "var(--font-mono)", fontSize: "11px",
  padding: "1px 7px", cursor: "pointer",
};

export default function FileBrowser({ projectPath, onOpenScript, onOpenScene, onFilesChange, onSceneChange }: Props) {
  const [tree, setTree] = useState<FileNode | null>(null);
  const [spriteEditorPath, setSpriteEditorPath] = useState<string | null>(null);
  const [spriteEditorComp, setSpriteEditorComp] = useState<AnimatedSpriteComp | null>(null);
  const [paletteDraft, setPaletteDraft] = useState<{ imagePath: string; name: string; cols: string; rows: string; tileW: string; tileH: string } | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set(["scripts", "scenes", "assets"]));
  const [selected, setSelected] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<{ path: string; value: string } | null>(null);
  const [creating, setCreating] = useState<CreateState | null>(null);
  const [dragOver, setDragOver] = useState<string | null>(null);
  const { show } = useContextMenu();
  const renameRef = useRef<HTMLInputElement>(null);
  const createRef = useRef<HTMLInputElement>(null);
  const dragCounterRef = useRef<Record<string, number>>({});

  const refresh = useCallback(async () => {
    if (!projectPath) return;
    try {
      const node = await invoke<FileNode>("list_project_tree", { projectPath });
      setTree(node);
      onFilesChange?.(flattenTree(node));
    } catch (e) {
      console.error("list_project_tree failed:", e);
    }
  }, [projectPath, onFilesChange]);

  useEffect(() => { refresh(); }, [refresh]);
  useEffect(() => { if (renaming) renameRef.current?.select(); }, [renaming]);
  useEffect(() => { if (creating) setTimeout(() => createRef.current?.focus(), 0); }, [creating]);

  const startCreate = (parentPath: string, type: CreateState["type"]) => {
    if (parentPath) setExpanded(prev => new Set([...prev, parentPath]));
    setCreating({ parentPath, type, value: "" });
  };

  const commitCreate = async () => {
    if (!creating || !projectPath) { setCreating(null); return; }
    const { parentPath, type, value } = creating;
    const name = value.trim();
    setCreating(null);
    if (!name) return;
    const relPath = parentPath ? `${parentPath}/${name}` : name;
    try {
      if (type === "script") {
        const created = await invoke<string>("new_script", { projectPath, relativePath: relPath });
        await refresh();
        setSelected(created);
        onOpenScript(created);
      } else if (type === "scene") {
        const created = await invoke<string>("new_scene_file", { projectPath, relativePath: relPath });
        await refresh();
        setSelected(created);
      } else {
        await invoke("create_folder", { projectPath, relativePath: relPath });
        await refresh();
        setExpanded(prev => new Set([...prev, relPath]));
      }
    } catch (e) {
      console.error("create failed:", e);
    }
  };

  const commitRename = async (node: FileNode, newName: string) => {
    setRenaming(null);
    if (!newName || newName === node.name || !projectPath) return;
    try {
      const newPath = await invoke<string>("rename_project_file", {
        projectPath,
        relativePath: node.path,
        newName,
      });
      if (selected === node.path) setSelected(newPath);
      refresh();
    } catch (e) {
      console.error("rename failed:", e);
    }
  };

  const handleDelete = async (node: FileNode) => {
    if (!projectPath) return;
    try {
      await invoke("delete_project_file", { projectPath, relativePath: node.path });
      if (selected === node.path) setSelected(null);
      refresh();
    } catch (e) {
      console.error("delete failed:", e);
    }
  };

  const openNode = (node: FileNode) => {
    if (node.kind === "script") onOpenScript(node.path);
    else if (node.kind === "scene") onOpenScene(node.path);
    else if (node.kind === "prefab") handleInstantiatePrefab(node.path);
  };

  const handleInstantiatePrefab = async (prefabPath: string) => {
    if (!projectPath) return;
    try {
      await invoke("instantiate_prefab", { projectPath, prefabPath });
      onSceneChange?.();
    } catch (e) {
      console.error("instantiate_prefab failed:", e);
    }
  };

  const commitCreatePalette = async () => {
    if (!paletteDraft || !projectPath) { setPaletteDraft(null); return; }
    const { imagePath, name, cols, rows, tileW, tileH } = paletteDraft;
    setPaletteDraft(null);
    try {
      await invoke("create_tile_palette", {
        projectPath,
        imageRelativePath: imagePath,
        name: name || imagePath.split("/").pop()?.replace(/\.[^.]+$/, "") || "palette",
        tilesetCols: parseInt(cols) || 4,
        tilesetRows: parseInt(rows) || 4,
        tileWidth: parseInt(tileW) || 16,
        tileHeight: parseInt(tileH) || 16,
        margin: 0,
        spacing: 0,
      });
      await refresh();
    } catch (e) {
      console.error("create_tile_palette failed:", e);
    }
  };

  const handleDrop = async (e: React.DragEvent, targetFolderPath: string) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current = {};
    setDragOver(null);
    const fromPath = e.dataTransfer.getData("text/plain");
    if (!fromPath || fromPath === targetFolderPath || !projectPath) return;
    if (targetFolderPath.startsWith(fromPath + "/")) return;
    try {
      await invoke("move_project_entry", {
        projectPath,
        fromRelative: fromPath,
        toFolderRelative: targetFolderPath,
      });
      refresh();
    } catch (e) {
      console.error("move failed:", e);
    }
  };

  const handleDragEnter = (e: React.DragEvent, path: string) => {
    e.preventDefault();
    dragCounterRef.current[path] = (dragCounterRef.current[path] ?? 0) + 1;
    setDragOver(path);
  };

  const handleDragLeave = (_e: React.DragEvent, path: string) => {
    dragCounterRef.current[path] = (dragCounterRef.current[path] ?? 1) - 1;
    if ((dragCounterRef.current[path] ?? 0) <= 0) {
      dragCounterRef.current[path] = 0;
      setDragOver(prev => (prev === path ? null : prev));
    }
  };

  const showContextMenu = (e: React.MouseEvent, node: FileNode) => {
    e.preventDefault();
    e.stopPropagation();
    const items = [
      ...(!node.is_dir && (node.kind === "script" || node.kind === "scene")
        ? [{ label: "Open", icon: "↗", onClick: () => openNode(node) }]
        : []),
      ...(!node.is_dir && node.kind === "prefab"
        ? [{ label: "Instantiate in Scene", icon: "◆", onClick: () => handleInstantiatePrefab(node.path) }]
        : []),
      ...(!node.is_dir && node.kind === "image"
        ? [
            { label: "Slice Spritesheet…", icon: "▣", onClick: () => {
              const initial: AnimatedSpriteComp = {
                texture_path: node.path,
                cols: 4, rows: 4, width: 64, height: 64,
                flip_x: false, flip_y: false,
                tint: [1, 1, 1, 1], clips: [], default_clip: "idle",
              };
              setSpriteEditorComp(initial);
              setSpriteEditorPath(node.path);
            }},
            { label: "Create Tile Palette…", icon: "◧", onClick: () => {
              const stem = node.name.replace(/\.[^.]+$/, "");
              setPaletteDraft({ imagePath: node.path, name: stem, cols: "4", rows: "4", tileW: "16", tileH: "16" });
            }},
          ]
        : []),
      ...(node.is_dir ? [
        { label: "New Script", icon: "⚡", onClick: () => startCreate(node.path, "script") },
        { label: "New Scene",  icon: "◈", onClick: () => startCreate(node.path, "scene") },
        { label: "New Folder", icon: "▷", onClick: () => startCreate(node.path, "folder") },
        { divider: true as const },
      ] : []),
      { label: "Rename", icon: "✎", onClick: () => setRenaming({ path: node.path, value: node.name }) },
      { divider: true as const },
      { label: "Delete", icon: "×", danger: true, onClick: () => handleDelete(node) },
    ];
    show(e.clientX, e.clientY, items);
  };

  const renderCreateInput = (depth: number) => {
    if (!creating) return null;
    const placeholder = creating.type === "folder" ? "folder-name"
      : creating.type === "script" ? "script.lua"
      : "scene.sindri";
    const icon = creating.type === "folder" ? "▷" : creating.type === "script" ? "⚡" : "◈";
    return (
      <div key="__create__" style={{
        display: "flex", alignItems: "center", gap: "7px",
        height: "22px", paddingLeft: `${depth * 14 + 8}px`, paddingRight: "6px",
      }}>
        <span style={{ fontSize: "10px", color: "var(--text-dim)", width: "12px", textAlign: "center" }}>
          {icon}
        </span>
        <input
          ref={createRef}
          value={creating.value}
          onChange={e => setCreating({ ...creating, value: e.target.value })}
          onKeyDown={e => {
            if (e.key === "Enter") commitCreate();
            if (e.key === "Escape") setCreating(null);
            e.stopPropagation();
          }}
          onBlur={commitCreate}
          placeholder={placeholder}
          style={inputStyle}
        />
      </div>
    );
  };

  const renderNode = (node: FileNode, depth: number): React.ReactNode => {
    const isExpanded = expanded.has(node.path);
    const isSelected = selected === node.path;
    const isDragOver = dragOver === node.path;
    const indent = depth * 14 + 8;
    const isCreatingHere = creating?.parentPath === node.path;

    if (node.is_dir) {
      return (
        <div key={node.path}>
          <div
            onClick={() => { setSelected(node.path); expanded.has(node.path) ? setExpanded(prev => { const s = new Set(prev); s.delete(node.path); return s; }) : setExpanded(prev => new Set([...prev, node.path])); }}
            onContextMenu={e => showContextMenu(e, node)}
            draggable
            onDragStart={e => { e.dataTransfer.setData("text/plain", node.path); e.stopPropagation(); }}
            onDragOver={e => e.preventDefault()}
            onDragEnter={e => handleDragEnter(e, node.path)}
            onDragLeave={e => handleDragLeave(e, node.path)}
            onDrop={e => handleDrop(e, node.path)}
            style={{
              display: "flex", alignItems: "center", gap: "5px",
              height: "22px", paddingLeft: `${indent}px`, paddingRight: "6px",
              cursor: "pointer", userSelect: "none",
              background: isDragOver ? "var(--accent-glow)" : isSelected ? "var(--bg-2)" : "none",
              outline: isDragOver ? "1px solid var(--accent-dim)" : "none",
              outlineOffset: "-1px",
            }}
          >
            <span style={{ fontSize: "8px", color: "var(--text-dim)", width: "10px", flexShrink: 0 }}>
              {isExpanded ? "▼" : "▶"}
            </span>
            <span style={{ fontSize: "11px", color: isDragOver ? "var(--accent)" : "var(--text-dim)" }}>▷</span>
            <span style={{
              flex: 1, fontSize: "11px", fontFamily: "var(--font-mono)",
              color: isSelected ? "var(--text-bright)" : "var(--text-muted)",
              overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
            }}>
              {node.name}
            </span>
          </div>

          {isExpanded && (
            <div>
              {node.children.map(child => renderNode(child, depth + 1))}
              {isCreatingHere && renderCreateInput(depth + 1)}
            </div>
          )}
        </div>
      );
    }

    // File
    return (
      <div
        key={node.path}
        draggable
        onDragStart={e => { e.dataTransfer.setData("text/plain", node.path); e.stopPropagation(); }}
        onClick={() => setSelected(node.path)}
        onDoubleClick={() => openNode(node)}
        onContextMenu={e => showContextMenu(e, node)}
        style={{
          display: "flex", alignItems: "center", gap: "7px",
          height: "22px", paddingLeft: `${indent}px`, paddingRight: "6px",
          cursor: "pointer", userSelect: "none",
          background: isSelected ? "var(--accent-glow)" : "none",
          borderLeft: `2px solid ${isSelected ? "var(--accent)" : "transparent"}`,
        }}
        onMouseEnter={e => { if (!isSelected) (e.currentTarget as HTMLDivElement).style.background = "var(--bg-2)"; }}
        onMouseLeave={e => { if (!isSelected) (e.currentTarget as HTMLDivElement).style.background = "none"; }}
      >
        <span style={{ fontSize: "10px", color: "var(--text-dim)", width: "12px", textAlign: "center", flexShrink: 0 }}>
          {FILE_ICON[node.kind] ?? "·"}
        </span>

        {renaming?.path === node.path ? (
          <input
            ref={renameRef}
            value={renaming.value}
            onChange={e => setRenaming({ ...renaming, value: e.target.value })}
            onKeyDown={e => {
              if (e.key === "Enter") commitRename(node, renaming.value.trim());
              if (e.key === "Escape") setRenaming(null);
              e.stopPropagation();
            }}
            onBlur={() => commitRename(node, renaming.value.trim())}
            onClick={e => e.stopPropagation()}
            style={inputStyle}
          />
        ) : (
          <span style={{
            flex: 1, fontSize: "11px", fontFamily: "var(--font-mono)",
            color: isSelected ? "var(--accent)" : "var(--text-bright)",
            overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
          }}>
            {node.name}
          </span>
        )}
      </div>
    );
  };

  if (!projectPath) {
    return (
      <div style={{ padding: "10px", color: "var(--text-dim)", fontSize: "11px" }}>
        No project open.
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden", minHeight: 0 }}>
      {/* Toolbar */}
      <div style={{
        height: "28px", borderBottom: "1px solid var(--border)",
        display: "flex", alignItems: "center", padding: "0 6px", gap: "4px", flexShrink: 0,
      }}>
        <button onClick={() => startCreate("scripts", "script")} title="New script" style={toolbarBtnStyle}>
          + script
        </button>
        <button onClick={() => startCreate("scenes", "scene")} title="New scene" style={toolbarBtnStyle}>
          + scene
        </button>
        <div style={{ flex: 1 }} />
        <button
          onClick={refresh}
          title="Refresh"
          style={{ background: "none", border: "none", color: "var(--text-dim)", cursor: "pointer", fontSize: "12px", padding: "2px 4px" }}
        >
          ↺
        </button>
      </div>

      {/* Tree */}
      <div
        style={{ flex: 1, overflow: "auto", padding: "4px 0" }}
        onDragOver={e => e.preventDefault()}
        onDrop={e => handleDrop(e, "")}
      >
        {!tree || tree.children.length === 0 ? (
          <div style={{ padding: "8px 10px", color: "var(--text-dim)", fontSize: "11px" }}>
            No files yet.
          </div>
        ) : (
          <>
            {tree.children.map(child => renderNode(child, 0))}
            {creating?.parentPath === "" && renderCreateInput(0)}
          </>
        )}
      </div>

      {paletteDraft && (
        <div style={{
          position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", zIndex: 1000,
          display: "flex", alignItems: "center", justifyContent: "center",
        }} onClick={() => setPaletteDraft(null)}>
          <div style={{
            background: "var(--paper)", border: "1px solid var(--rule)",
            padding: "20px 24px", minWidth: "280px", display: "flex", flexDirection: "column", gap: "12px",
          }} onClick={e => e.stopPropagation()}>
            <div style={{ fontSize: "13px", fontFamily: "var(--font-ui)", color: "var(--ink)", fontWeight: 500 }}>
              Create Tile Palette
            </div>
            <div style={{ fontSize: "10px", fontFamily: "var(--font-mono)", color: "var(--ink-3)" }}>
              {paletteDraft.imagePath}
            </div>
            {(["name", "cols", "rows", "tileW", "tileH"] as const).map(field => {
              const labels: Record<string, string> = { name: "name", cols: "columns", rows: "rows", tileW: "tile width", tileH: "tile height" };
              return (
                <div key={field} style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                  <span style={{ width: "72px", fontSize: "11px", color: "var(--ink-3)", fontFamily: "var(--font-ui)", flexShrink: 0 }}>{labels[field]}</span>
                  <input
                    value={paletteDraft[field]}
                    onChange={e => setPaletteDraft({ ...paletteDraft, [field]: e.target.value })}
                    onKeyDown={e => { if (e.key === "Enter") commitCreatePalette(); if (e.key === "Escape") setPaletteDraft(null); }}
                    style={{ flex: 1, height: "22px", background: "var(--bg-1)", border: "1px solid var(--rule)", color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "0 6px", outline: "none" }}
                  />
                </div>
              );
            })}
            <div style={{ display: "flex", justifyContent: "flex-end", gap: "8px", marginTop: "4px" }}>
              <button onClick={() => setPaletteDraft(null)} style={{ background: "none", border: "1px solid var(--rule)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "3px 10px", cursor: "pointer" }}>Cancel</button>
              <button onClick={commitCreatePalette} style={{ background: "var(--moss)", border: "none", color: "var(--paper)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "3px 10px", cursor: "pointer" }}>Create</button>
            </div>
          </div>
        </div>
      )}

      {spriteEditorPath && spriteEditorComp && (
        <AnimClipEditor
          comp={spriteEditorComp}
          saveLabel="Save Animation File"
          onClose={() => { setSpriteEditorPath(null); setSpriteEditorComp(null); }}
          onSave={(partial) => setSpriteEditorComp(prev => prev ? { ...prev, ...partial } : prev)}
          onCommit={async (partial) => {
            if (!projectPath || !spriteEditorComp) return;
            const final = { ...spriteEditorComp, ...partial };
            const baseName = spriteEditorPath.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "sprite";
            const dir = spriteEditorPath.includes("/")
              ? spriteEditorPath.substring(0, spriteEditorPath.lastIndexOf("/"))
              : "";
            const relPath = dir ? `${dir}/${baseName}` : baseName;
            try {
              await invoke("write_anim_file", {
                projectPath,
                relativePath: relPath,
                content: JSON.stringify(final, null, 2),
              });
            } catch (e) {
              console.error("Failed to save animation file:", e);
            }
            setSpriteEditorPath(null);
            setSpriteEditorComp(null);
          }}
        />
      )}
    </div>
  );
}
