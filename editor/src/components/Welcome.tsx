import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface WelcomeProps {
  onProjectOpen: () => void;
}

interface ProjectInfo {
  name: string;
  path: string;
  version: string;
}

export default function Welcome({ onProjectOpen }: WelcomeProps) {
  const [projectName, setProjectName] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [isLoadingProjects, setIsLoadingProjects] = useState(true);

  // Load projects list on mount
  useEffect(() => {
    const loadProjects = async () => {
      try {
        const projectList = await invoke<ProjectInfo[]>("project_list");
        setProjects(projectList);
      } catch (e) {
        console.error("Failed to load projects:", e);
      } finally {
        setIsLoadingProjects(false);
      }
    };
    loadProjects();
  }, []);

  const handleCreateProject = async () => {
    if (!projectName.trim()) {
      setError("Project name is required");
      return;
    }

    setIsCreating(true);
    setError(null);

    try {
      await invoke("project_create", {
        name: projectName.trim(),
      });
      // Reload projects list
      const projectList = await invoke<ProjectInfo[]>("project_list");
      setProjects(projectList);
      onProjectOpen();
    } catch (e) {
      setError(`Failed to create project: ${e}`);
      setIsCreating(false);
    }
  };

  const handleOpenProject = async (projectPath: string) => {
    try {
      await invoke("project_open", { path: projectPath });
      onProjectOpen();
    } catch (e) {
      setError(`Failed to open project: ${e}`);
      console.error("Failed to open project:", e);
    }
  };

  return (
    <div className="flex items-center justify-center h-full w-full bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900">
      <div className="bg-gray-800 rounded-lg shadow-2xl p-8 w-full max-w-2xl border border-gray-700">
        <div className="text-center mb-8">
          <h1 className="text-4xl font-bold text-white mb-2">Forge2D Editor</h1>
          <p className="text-gray-400">Create or open a project to get started</p>
          <p className="text-xs text-gray-500 mt-2">
            Projects are stored in Documents/Forge2D
          </p>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-red-900/50 border border-red-700 rounded text-red-200 text-sm">
            {error}
          </div>
        )}

        <div className="grid grid-cols-2 gap-6">
          {/* Left: Create New Project */}
          <div className="space-y-4">
            <h2 className="text-xl font-semibold text-white">Create New Project</h2>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">
                Project Name
              </label>
              <input
                type="text"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && projectName.trim() && !isCreating) {
                    handleCreateProject();
                  }
                }}
                placeholder="My Game"
                className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
                disabled={isCreating}
              />
            </div>

            <button
              onClick={handleCreateProject}
              disabled={isCreating || !projectName.trim()}
              className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed rounded text-white font-medium transition-colors"
            >
              {isCreating ? "Creating..." : "Create Project"}
            </button>
          </div>

          {/* Right: Recent Projects */}
          <div className="space-y-4">
            <h2 className="text-xl font-semibold text-white">Recent Projects</h2>
            
            {isLoadingProjects ? (
              <div className="text-gray-400 text-sm">Loading projects...</div>
            ) : projects.length === 0 ? (
              <div className="text-gray-400 text-sm">
                No projects yet. Create one to get started!
              </div>
            ) : (
              <div className="space-y-2 max-h-64 overflow-y-auto">
                {projects.map((project) => (
                  <button
                    key={project.path}
                    onClick={() => handleOpenProject(project.path)}
                    className="w-full px-3 py-2 bg-gray-700 hover:bg-gray-600 border border-gray-600 rounded text-left text-sm transition-colors"
                  >
                    <div className="font-medium text-white">{project.name}</div>
                    <div className="text-xs text-gray-400 mt-0.5">
                      v{project.version}
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="mt-8 text-center text-xs text-gray-500">
          Forge2D Editor v0.1.0
        </div>
      </div>
    </div>
  );
}
