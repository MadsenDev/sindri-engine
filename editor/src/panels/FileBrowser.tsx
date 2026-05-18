import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useContextMenu } from "../components/ContextMenu";

interface ProjectFile {
  path: string;
  kind: string;
  name: string;
}

interface Props {
  projectPath: string | null;
  onOpenScript: (path: string) => void;
  onOpenScene: (path: string) => void;
  onFilesChange?: (files: ProjectFile[]) => void;
}

const KIND_ICON: Record<string, string> = {
  script: "⚡",
  scene:  "◈",
  image:  "▣",
  audio:  "♪",
  other:  "·",
};

const KIND_ORDER = ["script", "scene", "image", "audio", "other"];
const KIND_LABEL: Record<string, string> = {
  script: "Scripts",
  scene:  "Scenes",
  image:  "Images",
  audio:  "Audio",
  other:  "Other",
};

export default function FileBrowser({ projectPath, onOpenScript, onOpenScene, onFilesChange }: Props) {
  const [files, setFiles] = useState<ProjectFile[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [renamingPath, setRenamingPath] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [newScriptOpen, setNewScriptOpen] = useState(false);
  const [newScriptName, setNewScriptName] = useState("");
  const renameRef = useRef<HTMLInputElement>(null);
  const newScriptRef = useRef<HTMLInputElement>(null);
  const { show } = useContextMenu();

  const refresh = async () => {
    if (!projectPath) return;
    try {
      const result = await invoke<ProjectFile[]>("list_project_files", { projectPath });
      setFiles(result);
      onFilesChange?.(result);
    } catch {}
  };

  useEffect(() => { refresh(); }, [projectPath]);

  useEffect(() => {
    if (renamingPath) renameRef.current?.select();
  }, [renamingPath]);

  useEffect(() => {
    if (newScriptOpen) {
      setNewScriptName("");
      setTimeout(() => newScriptRef.current?.focus(), 0);
    }
  }, [newScriptOpen]);

  const handleOpen = (file: ProjectFile) => {
    if (file.kind === "script") {
      // file.path is project-relative ("scripts/foo.lua") but the engine's
      // /script route resolves relative to scripts_root, so strip the prefix.
      const scriptRelative = file.path.replace(/^scripts\//, "");
      onOpenScript(scriptRelative);
      return;
    }
    if (file.kind === "scene") {
      onOpenScene(file.path);
    }
  };

  const handleDelete = async (file: ProjectFile) => {
    if (!projectPath) return;
    try {
      await invoke("delete_project_file", { projectPath, relativePath: file.path });
      if (selected === file.path) setSelected(null);
      refresh();
    } catch (err) {
      console.error("delete failed:", err);
    }
  };

  const startRename = (file: ProjectFile) => {
    setRenamingPath(file.path);
    setRenameValue(file.name);
  };

  const commitRename = async (file: ProjectFile) => {
    const newName = renameValue.trim();
    setRenamingPath(null);
    if (!newName || newName === file.name || !projectPath) return;
    try {
      const newPath = await invoke<string>("rename_project_file", {
        projectPath,
        relativePath: file.path,
        newName,
      });
      if (selected === file.path) setSelected(newPath);
      refresh();
    } catch (err) {
      console.error("rename failed:", err);
    }
  };

  const commitNewScript = async () => {
    const name = newScriptName.trim();
    setNewScriptOpen(false);
    if (!name || !projectPath) return;
    try {
      const path = await invoke<string>("new_script", { projectPath, name });
      await refresh();
      setSelected(path);
      onOpenScript(path.replace(/^scripts\//, ""));
    } catch (err) {
      console.error("new script failed:", err);
    }
  };

  const handleContextMenu = (e: React.MouseEvent, file: ProjectFile) => {
    e.preventDefault();
    e.stopPropagation();
    const items = [
      ...(file.kind === "script" || file.kind === "scene" ? [{ label: "Open", icon: "↗", onClick: () => handleOpen(file) }] : []),
      { label: "Rename", icon: "✎", onClick: () => startRename(file) },
      { divider: true as const },
      { label: "Delete", icon: "×", danger: true, onClick: () => handleDelete(file) },
    ];
    show(e.clientX, e.clientY, items);
  };

  // Group files by kind
  const byKind: Record<string, ProjectFile[]> = {};
  for (const f of files) {
    (byKind[f.kind] ??= []).push(f);
  }

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
        <button
          onClick={() => setNewScriptOpen(v => !v)}
          title="New script"
          style={{
            background: newScriptOpen ? "var(--accent-glow)" : "none",
            border: `1px solid ${newScriptOpen ? "var(--accent-dim)" : "var(--border)"}`,
            borderRadius: "var(--radius)", color: newScriptOpen ? "var(--accent)" : "var(--text-muted)",
            fontFamily: "var(--font-mono)", fontSize: "11px",
            padding: "1px 7px", cursor: "pointer",
          }}
        >+ script</button>
        <div style={{ flex: 1 }} />
        <button
          onClick={refresh}
          title="Refresh"
          style={{
            background: "none", border: "none", color: "var(--text-dim)",
            cursor: "pointer", fontSize: "12px", padding: "2px 4px",
          }}
        >↺</button>
      </div>

      {/* New script input */}
      {newScriptOpen && (
        <div style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", flexShrink: 0 }}>
          <input
            ref={newScriptRef}
            value={newScriptName}
            onChange={e => setNewScriptName(e.target.value)}
            onKeyDown={e => {
              if (e.key === "Enter") commitNewScript();
              if (e.key === "Escape") setNewScriptOpen(false);
            }}
            onBlur={() => setNewScriptOpen(false)}
            placeholder="script-name.lua"
            style={{
              width: "100%", height: "22px", background: "var(--bg-3)",
              border: "1px solid var(--accent)", borderRadius: "var(--radius)",
              color: "var(--text-bright)", fontFamily: "var(--font-mono)",
              fontSize: "11px", padding: "0 6px", outline: "none",
            }}
          />
        </div>
      )}

      {/* File list */}
      <div style={{ flex: 1, overflow: "auto", padding: "4px 0" }}>
        {files.length === 0 ? (
          <div style={{ padding: "8px 10px", color: "var(--text-dim)", fontSize: "11px" }}>
            No files yet.
          </div>
        ) : (
          KIND_ORDER.filter(k => byKind[k]?.length).map(kind => (
            <div key={kind}>
              {/* Section header */}
              <div style={{
                padding: "4px 10px 2px",
                fontSize: "9px", letterSpacing: "0.1em", textTransform: "uppercase",
                color: "var(--text-dim)", fontFamily: "var(--font-ui)", fontWeight: 600,
              }}>
                {KIND_LABEL[kind]}
              </div>

              {byKind[kind].map(file => (
                <div
                  key={file.path}
                  onClick={() => setSelected(file.path)}
                  onDoubleClick={() => handleOpen(file)}
                  onContextMenu={e => handleContextMenu(e, file)}
                  style={{
                    display: "flex", alignItems: "center", gap: "7px",
                    height: "24px", padding: "0 10px",
                    background: selected === file.path ? "var(--accent-glow)" : "none",
                    borderLeft: `2px solid ${selected === file.path ? "var(--accent)" : "transparent"}`,
                    cursor: "pointer",
                  }}
                  onMouseEnter={e => { if (selected !== file.path) (e.currentTarget as HTMLElement).style.background = "var(--bg-2)"; }}
                  onMouseLeave={e => { if (selected !== file.path) (e.currentTarget as HTMLElement).style.background = "none"; }}
                >
                  <span style={{ fontSize: "10px", color: "var(--text-dim)", width: "12px", textAlign: "center", flexShrink: 0 }}>
                    {KIND_ICON[file.kind] ?? "·"}
                  </span>

                  {renamingPath === file.path ? (
                    <input
                      ref={renameRef}
                      value={renameValue}
                      onChange={e => setRenameValue(e.target.value)}
                      onKeyDown={e => {
                        if (e.key === "Enter") commitRename(file);
                        if (e.key === "Escape") setRenamingPath(null);
                        e.stopPropagation();
                      }}
                      onBlur={() => commitRename(file)}
                      onClick={e => e.stopPropagation()}
                      style={{
                        flex: 1, height: "18px", background: "var(--bg-3)",
                        border: "1px solid var(--accent)", borderRadius: "var(--radius)",
                        color: "var(--text-bright)", fontFamily: "var(--font-mono)",
                        fontSize: "10px", padding: "0 4px", outline: "none",
                      }}
                    />
                  ) : (
                    <span style={{
                      flex: 1, fontSize: "11px",
                      color: selected === file.path ? "var(--accent)" : "var(--text-bright)",
                      overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                      fontFamily: "var(--font-mono)",
                    }}>
                      {file.name}
                    </span>
                  )}
                </div>
              ))}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
