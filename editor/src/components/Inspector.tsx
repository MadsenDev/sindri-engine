import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface ComponentFieldInfo {
  name: string;
  type_name: string;
  value: any;
}

interface SpriteData {
  texture_handle: number;
  texture_path: string | null;
  texture_size: [number, number] | null;
  tint: [number, number, number, number];
  sprite_scale: [number, number];
}

interface CameraData {
  active: boolean;
  zoom: number;
  offset: [number, number];
  rotation: number;
}

interface InspectorProps {
  selectedEntityId: number | null;
  refreshTrigger?: number; // Increment this to force refresh
}

export default function Inspector({ selectedEntityId, refreshTrigger }: InspectorProps) {
  const [componentTypes, setComponentTypes] = useState<string[]>([]);
  const [attachableTypes, setAttachableTypes] = useState<string[]>([]);
  const [attachedComponents, setAttachedComponents] = useState<string[]>([]);
  const [componentToAdd, setComponentToAdd] = useState<string>("");
  const [fields, setFields] = useState<Record<string, ComponentFieldInfo[]>>({});
  const [spriteData, setSpriteData] = useState<SpriteData | null>(null);
  const [cameraData, setCameraData] = useState<CameraData | null>(null);

  useEffect(() => {
    if (selectedEntityId === null) {
      setFields({});
      setAttachedComponents([]);
      setSpriteData(null);
      setCameraData(null);
      return;
    }

    const loadComponentTypes = async () => {
      const types = await invoke<string[]>("component_types");
      const attachable = await invoke<string[]>("component_attachable_types");
      const attached = await invoke<string[]>("entity_components", {
        entityId: selectedEntityId,
      });
      setComponentTypes(types);
      setAttachableTypes(attachable);
      setAttachedComponents(attached);
      setComponentToAdd((prev) => prev || attachable[0] || "");

      if (attached.includes("SpriteComponent")) {
        const sprite = await invoke<SpriteData | null>("sprite_get", {
          entityId: selectedEntityId,
        });
        setSpriteData(sprite);
      } else {
        setSpriteData(null);
      }

      if (attached.includes("CameraComponent")) {
        const cam = await invoke<CameraData | null>("camera_get", {
          entityId: selectedEntityId,
        });
        setCameraData(cam);
      } else {
        setCameraData(null);
      }

      // Load fields for each component type
      const fieldMap: Record<string, ComponentFieldInfo[]> = {};
      for (const type of types) {
        const componentFields = await invoke<ComponentFieldInfo[] | null>(
          "component_fields",
          { entityId: selectedEntityId, componentType: type }
        );
        if (componentFields) {
          fieldMap[type] = componentFields;
        }
      }
      setFields(fieldMap);
    };

    loadComponentTypes();
  }, [selectedEntityId, refreshTrigger]);

  const handleFieldChange = async (
    componentType: string,
    fieldName: string,
    value: any
  ) => {
    if (selectedEntityId === null) return;

    await invoke("component_set_field", {
      entityId: selectedEntityId,
      componentType,
      fieldName,
      value,
    });

    // Reload fields
    const componentFields = await invoke<ComponentFieldInfo[] | null>(
      "component_fields",
      { entityId: selectedEntityId, componentType }
    );
    if (componentFields) {
      setFields((prev) => ({ ...prev, [componentType]: componentFields }));
    }
  };

  const handleAddComponent = async () => {
    if (selectedEntityId === null || !componentToAdd) return;
    await invoke("component_add", {
      entityId: selectedEntityId,
      componentType: componentToAdd,
    });
    const attached = await invoke<string[]>("entity_components", {
      entityId: selectedEntityId,
    });
    setAttachedComponents(attached);
    if (componentToAdd === "SpriteComponent") {
      const sprite = await invoke<SpriteData | null>("sprite_get", {
        entityId: selectedEntityId,
      });
      setSpriteData(sprite);
    }
    if (componentToAdd === "CameraComponent") {
      const cam = await invoke<CameraData | null>("camera_get", {
        entityId: selectedEntityId,
      });
      setCameraData(cam);
    }
  };

  const handleRemoveComponent = async (componentType: string) => {
    if (selectedEntityId === null) return;
    await invoke("component_remove", {
      entityId: selectedEntityId,
      componentType,
    });
    const attached = await invoke<string[]>("entity_components", {
      entityId: selectedEntityId,
    });
    setAttachedComponents(attached);
    setFields((prev) => {
      const next = { ...prev };
      delete next[componentType];
      return next;
    });
    if (componentType === "SpriteComponent") {
      setSpriteData(null);
    }
    if (componentType === "CameraComponent") {
      setCameraData(null);
    }
  };

  const handleCameraChange = async (next: CameraData) => {
    if (selectedEntityId === null) return;
    await invoke("camera_set", {
      entityId: selectedEntityId,
      active: next.active,
      zoom: next.zoom,
      offset: next.offset,
      rotation: next.rotation,
    });
    const cam = await invoke<CameraData | null>("camera_get", {
      entityId: selectedEntityId,
    });
    setCameraData(cam);
  };

  const handlePickTexture = async () => {
    if (selectedEntityId === null) return;
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
    let importedPath = filePath;
    try {
      importedPath = await invoke<string>("asset_import_texture", {
        path: filePath,
      });
    } catch (e) {
      console.error("Texture import failed:", e);
    }
    await invoke("sprite_set_texture_path", {
      entityId: selectedEntityId,
      path: importedPath,
    });
    const sprite = await invoke<SpriteData | null>("sprite_get", {
      entityId: selectedEntityId,
    });
    setSpriteData(sprite);
  };

  const handleClearTexture = async () => {
    if (selectedEntityId === null) return;
    await invoke("sprite_set_texture_path", {
      entityId: selectedEntityId,
      path: "",
    });
    const sprite = await invoke<SpriteData | null>("sprite_get", {
      entityId: selectedEntityId,
    });
    setSpriteData(sprite);
  };

  if (selectedEntityId === null) {
    return (
      <div className="p-4 text-gray-400 text-sm">
        Select an entity to inspect
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-4">
      <div className="mb-4">
        <div className="text-xs text-gray-400 mb-1">Entity ID</div>
        <div className="font-mono text-sm">{selectedEntityId}</div>
      </div>

      {componentTypes.length === 0 && attachedComponents.length === 0 ? (
        <div className="text-gray-400 text-sm">No components</div>
      ) : (
        attachedComponents.map((type) => {
          const componentFields = fields[type];

          return (
            <div key={type} className="mb-6">
              <div className="flex items-center justify-between mb-2">
                <div className="text-sm font-semibold text-blue-400">{type}</div>
                <button
                  onClick={() => handleRemoveComponent(type)}
                  className="text-xs px-2 py-1 rounded bg-red-600/20 text-red-300 hover:bg-red-600/30"
                >
                  Remove
                </button>
              </div>
              <div className="space-y-3">
                {type === "SpriteComponent" && (
                  <div className="rounded border border-gray-700/60 bg-gray-800/40 p-2">
                    <div className="text-xs text-gray-400 mb-2">
                      Texture
                    </div>
                    <div className="flex gap-2">
                      <button
                        onClick={handlePickTexture}
                        className="px-2 py-1 rounded bg-blue-600/80 text-xs"
                      >
                        Pick
                      </button>
                      <button
                        onClick={handleClearTexture}
                        className="px-2 py-1 rounded bg-gray-700 text-xs"
                      >
                        Clear
                      </button>
                      <div className="flex-1 truncate text-xs text-gray-400">
                        {spriteData?.texture_path || "No texture set"}
                      </div>
                    </div>
                    {spriteData?.texture_path && (
                      <div className="mt-2">
                        <img
                          src={spriteData.texture_path}
                          alt="Sprite texture"
                          className="max-h-32 max-w-full rounded border border-gray-700 object-contain bg-black/20"
                        />
                      </div>
                    )}
                  </div>
                )}
                {type === "CameraComponent" && (
                  <div className="rounded border border-gray-700/60 bg-gray-800/40 p-2">
                    <div className="text-xs text-gray-400 mb-2">
                      Camera
                    </div>
                    {cameraData ? (
                      <div className="space-y-2">
                        <label className="flex items-center gap-2 text-xs text-gray-300">
                          <input
                            type="checkbox"
                            checked={cameraData.active}
                            onChange={(e) =>
                              handleCameraChange({
                                ...cameraData,
                                active: e.target.checked,
                              })
                            }
                          />
                          Active
                        </label>
                        <div>
                          <label className="block text-xs text-gray-400 mb-1">
                            Zoom
                          </label>
                          <input
                            type="number"
                            step="0.05"
                            value={cameraData.zoom}
                            onChange={(e) =>
                              handleCameraChange({
                                ...cameraData,
                                zoom: parseFloat(e.target.value) || 0.01,
                              })
                            }
                            className="w-full px-2 py-1 bg-gray-700 rounded text-sm"
                          />
                        </div>
                        <div>
                          <label className="block text-xs text-gray-400 mb-1">
                            Rotation
                          </label>
                          <input
                            type="number"
                            step="0.05"
                            value={cameraData.rotation}
                            onChange={(e) =>
                              handleCameraChange({
                                ...cameraData,
                                rotation: parseFloat(e.target.value) || 0,
                              })
                            }
                            className="w-full px-2 py-1 bg-gray-700 rounded text-sm"
                          />
                        </div>
                        <div>
                          <label className="block text-xs text-gray-400 mb-1">
                            Offset
                          </label>
                          <div className="grid grid-cols-2 gap-2">
                            <input
                              type="number"
                              step="1"
                              value={cameraData.offset[0]}
                              onChange={(e) =>
                                handleCameraChange({
                                  ...cameraData,
                                  offset: [
                                    parseFloat(e.target.value) || 0,
                                    cameraData.offset[1],
                                  ],
                                })
                              }
                              className="px-2 py-1 bg-gray-700 rounded text-sm"
                            />
                            <input
                              type="number"
                              step="1"
                              value={cameraData.offset[1]}
                              onChange={(e) =>
                                handleCameraChange({
                                  ...cameraData,
                                  offset: [
                                    cameraData.offset[0],
                                    parseFloat(e.target.value) || 0,
                                  ],
                                })
                              }
                              className="px-2 py-1 bg-gray-700 rounded text-sm"
                            />
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div className="text-xs text-gray-400">
                        No camera data
                      </div>
                    )}
                  </div>
                )}
                {!componentFields || componentFields.length === 0 ? (
                  <div className="text-xs text-gray-400">
                    No editable fields
                  </div>
                ) : (
                  componentFields.map((field) => (
                    <div key={field.name}>
                      <label className="block text-xs text-gray-400 mb-1">
                        {field.name} ({field.type_name})
                      </label>
                      {field.type_name === "f32" ? (
                        <input
                          type="number"
                          step="0.1"
                          value={field.value as number}
                          onChange={(e) =>
                            handleFieldChange(
                              type,
                              field.name,
                              parseFloat(e.target.value) || 0
                            )
                          }
                          className="w-full px-2 py-1 bg-gray-700 rounded text-sm"
                        />
                      ) : field.type_name === "Vec2" ? (
                        <div className="grid grid-cols-2 gap-2">
                          <input
                            type="number"
                            step="0.1"
                            value={(field.value as any)?.x || 0}
                            onChange={(e) =>
                              handleFieldChange(type, field.name, {
                                x: parseFloat(e.target.value) || 0,
                                y: (field.value as any)?.y || 0,
                              })
                            }
                            placeholder="X"
                            className="px-2 py-1 bg-gray-700 rounded text-sm"
                          />
                          <input
                            type="number"
                            step="0.1"
                            value={(field.value as any)?.y || 0}
                            onChange={(e) =>
                              handleFieldChange(type, field.name, {
                                x: (field.value as any)?.x || 0,
                                y: parseFloat(e.target.value) || 0,
                              })
                            }
                            placeholder="Y"
                            className="px-2 py-1 bg-gray-700 rounded text-sm"
                          />
                        </div>
                      ) : (
                        <div className="px-2 py-1 bg-gray-700 rounded text-sm text-gray-300">
                          {JSON.stringify(field.value)}
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          );
        })
      )}

      <div className="mt-6 border-t border-gray-700 pt-4">
        <div className="text-xs text-gray-400 mb-2">Add Component</div>
        <div className="flex gap-2">
          <select
            value={componentToAdd}
            onChange={(e) => setComponentToAdd(e.target.value)}
            className="flex-1 px-2 py-1 bg-gray-700 rounded text-sm"
          >
            {attachableTypes.map((type) => (
              <option key={type} value={type}>
                {type}
              </option>
            ))}
          </select>
          <button
            onClick={handleAddComponent}
            className="px-3 py-1 rounded bg-blue-600 text-sm"
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}
