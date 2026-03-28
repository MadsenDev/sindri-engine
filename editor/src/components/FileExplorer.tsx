import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileNode[];
}

interface ProjectFileTree {
  scenes: FileNode;
  assets: FileNode;
}

interface FileExplorerProps {
  refreshToken: number;
  onOpenScene: (path: string) => void;
  onInstantiatePrefab: (path: string) => void;
}

const imageExtensions = new Set(["png", "jpg", "jpeg", "gif", "webp"]);

function fileExtension(path: string) {
  return path.split(".").pop()?.toLowerCase() ?? "";
}

function FileNodeView({
  node,
  level = 0,
  onOpenScene,
  onInstantiatePrefab,
}: {
  node: FileNode;
  level?: number;
  onOpenScene: (path: string) => void;
  onInstantiatePrefab: (path: string) => void;
}) {
  const [expanded, setExpanded] = useState(true);
  const isDirectory = node.is_dir;
  const extension = fileExtension(node.path);
  const isImage = imageExtensions.has(extension);
  const isScene = extension === "json" && /[\\/]scenes[\\/]/.test(node.path);
  const isPrefab = extension === "prefab" || node.path.endsWith(".prefab.json");

  const handleOpen = () => {
    if (isScene) {
      onOpenScene(node.path);
    } else if (isPrefab) {
      onInstantiatePrefab(node.path);
    }
  };

  return (
    <div className="file-node" style={{ paddingLeft: `${level * 14}px` }}>
      <div
        className="file-node-row"
        onClick={() => isDirectory && setExpanded(!expanded)}
        onDoubleClick={() => !isDirectory && handleOpen()}
        draggable={!isDirectory}
        onDragStart={(event) => {
          if (isDirectory) return;
          event.dataTransfer.setData(
            "application/x-forge2d-asset",
            JSON.stringify({
              path: node.path,
              kind: isPrefab ? "prefab" : isImage ? "texture" : "file",
            })
          );
          event.dataTransfer.effectAllowed = "copy";
        }}
      >
        {isDirectory ? (
          <button
            className="file-toggle"
            onClick={(e) => {
              e.stopPropagation();
              setExpanded(!expanded);
            }}
            aria-label={expanded ? "Collapse" : "Expand"}
          >
            {expanded ? "▼" : "▶"}
          </button>
        ) : (
          <span className="file-toggle placeholder" />
        )}
        <div className="file-label">
          <span className="file-icon">{isDirectory ? "📁" : isImage ? "🖼️" : isPrefab ? "🧩" : isScene ? "🎬" : "📄"}</span>
          {!isDirectory && isImage && (
            <img
              src={node.path}
              alt=""
              className="h-8 w-8 rounded border border-gray-700 object-cover bg-black/30"
            />
          )}
          <span className="file-name">{node.name}</span>
        </div>
      </div>

      {isDirectory && expanded && node.children && node.children.length > 0 && (
        <div className="file-children">
          {node.children.map((child) => (
            <FileNodeView
              key={child.path}
              node={child}
              level={level + 1}
              onOpenScene={onOpenScene}
              onInstantiatePrefab={onInstantiatePrefab}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export default function FileExplorer({
  refreshToken,
  onOpenScene,
  onInstantiatePrefab,
}: FileExplorerProps) {
  const [tree, setTree] = useState<ProjectFileTree | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    const loadTree = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const data = await invoke<ProjectFileTree>("project_files_tree");
        setTree(data);
      } catch (e) {
        console.error("Failed to load project files", e);
        setError("Unable to load project files");
      } finally {
        setIsLoading(false);
      }
    };

    loadTree();
  }, [refreshToken]);

  if (isLoading) {
    return <p className="text-gray-400 text-sm p-2">Loading files...</p>;
  }

  if (error) {
    return <p className="text-red-400 text-sm p-2">{error}</p>;
  }

  if (!tree) {
    return <p className="text-gray-400 text-sm p-2">No files found.</p>;
  }

  return (
    <div className="h-full overflow-y-auto flex flex-col gap-3">
      {[tree.scenes, tree.assets].map((root) => (
        <div key={root.path} className="file-section">
          <div className="file-section-header">
            <span className="pill muted">{root.name}</span>
            <span className="text-gray-400 text-xs">{root.children?.length ?? 0} items</span>
          </div>
          <div className="file-section-body">
            <FileNodeView
              node={root}
              onOpenScene={onOpenScene}
              onInstantiatePrefab={onInstantiatePrefab}
            />
            {(!root.children || root.children.length === 0) && (
              <p className="text-gray-500 text-sm px-2 py-1">Empty folder</p>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
