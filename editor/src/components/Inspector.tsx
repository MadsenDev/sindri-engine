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

interface ScriptTagData {
  tag: string;
}

interface InspectorSnapshot {
  component_types: string[];
  attachable_types: string[];
  attached_components: string[];
  fields: Record<string, ComponentFieldInfo[]>;
  sprite_data: SpriteData | null;
  camera_data: CameraData | null;
  script_data: ScriptTagData | null;
}

type DraftValue = string | { x: string; y: string };

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
  const [scriptData, setScriptData] = useState<ScriptTagData | null>(null);
  const [fieldDrafts, setFieldDrafts] = useState<Record<string, DraftValue>>({});
  const [cameraDraft, setCameraDraft] = useState<CameraData | null>(null);
  const [scriptDraft, setScriptDraft] = useState("");

  const fieldKey = (componentType: string, fieldName: string) =>
    `${componentType}:${fieldName}`;

  useEffect(() => {
    if (selectedEntityId === null) {
      setFields({});
      setAttachedComponents([]);
      setSpriteData(null);
      setCameraData(null);
      setScriptData(null);
      setFieldDrafts({});
      setCameraDraft(null);
      setScriptDraft("");
      return;
    }

    const loadComponentTypes = async () => {
      const snapshot = await invoke<InspectorSnapshot>("inspector_snapshot", {
        entityId: selectedEntityId,
      });
      setComponentTypes(snapshot.component_types);
      setAttachableTypes(snapshot.attachable_types);
      setAttachedComponents(snapshot.attached_components);
      setComponentToAdd((prev) => prev || snapshot.attachable_types[0] || "");
      setFields(snapshot.fields);
      setSpriteData(snapshot.sprite_data);
      setCameraData(snapshot.camera_data);
      setScriptData(snapshot.script_data);
      setCameraDraft(snapshot.camera_data);
      setScriptDraft(snapshot.script_data?.tag ?? "");
      const nextDrafts: Record<string, DraftValue> = {};
      for (const [componentType, componentFields] of Object.entries(snapshot.fields)) {
        for (const field of componentFields) {
          const key = fieldKey(componentType, field.name);
          if (field.type_name === "Vec2") {
            nextDrafts[key] = {
              x: String((field.value as any)?.x ?? 0),
              y: String((field.value as any)?.y ?? 0),
            };
          } else if (field.type_name === "f32") {
            nextDrafts[key] = String(field.value ?? 0);
          }
        }
      }
      setFieldDrafts(nextDrafts);
    };

    loadComponentTypes().catch((error) => {
      console.error("Failed to load inspector snapshot", error);
    });
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
      const draftKey = fieldKey(componentType, fieldName);
      const updatedField = componentFields.find((field) => field.name === fieldName);
      if (updatedField) {
        setFieldDrafts((prev) => ({
          ...prev,
          [draftKey]:
            updatedField.type_name === "Vec2"
              ? {
                  x: String((updatedField.value as any)?.x ?? 0),
                  y: String((updatedField.value as any)?.y ?? 0),
                }
              : String(updatedField.value ?? 0),
        }));
      }
    }
  };

  const commitFieldDraft = async (componentType: string, field: ComponentFieldInfo) => {
    const key = fieldKey(componentType, field.name);
    const draft = fieldDrafts[key];
    if (draft === undefined) return;
    if (field.type_name === "f32" && typeof draft === "string") {
      await handleFieldChange(componentType, field.name, parseFloat(draft) || 0);
      return;
    }
    if (field.type_name === "Vec2" && typeof draft !== "string") {
      await handleFieldChange(componentType, field.name, {
        x: parseFloat(draft.x) || 0,
        y: parseFloat(draft.y) || 0,
      });
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
      setCameraDraft(cam);
    }
    if (componentToAdd === "ScriptTag") {
      const script = await invoke<ScriptTagData | null>("script_tag_get", {
        entityId: selectedEntityId,
      });
      setScriptData(script);
      setScriptDraft(script?.tag ?? "");
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
      setCameraDraft(null);
    }
    if (componentType === "ScriptTag") {
      setScriptData(null);
      setScriptDraft("");
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
    setCameraDraft(cam);
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

  const handlePickScript = async () => {
    if (selectedEntityId === null) return;
    const filePath = await open({
      filters: [
        {
          name: "Script",
          extensions: ["lua", "rhai"],
        },
      ],
    });
    if (!filePath || typeof filePath !== "string") {
      return;
    }
    await invoke("script_tag_set", {
      entityId: selectedEntityId,
      tag: filePath,
    });
    const script = await invoke<ScriptTagData | null>("script_tag_get", {
      entityId: selectedEntityId,
    });
    setScriptData(script);
    setScriptDraft(script?.tag ?? "");
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

  const commitScriptDraft = async () => {
    if (selectedEntityId === null) return;
    await invoke("script_tag_set", {
      entityId: selectedEntityId,
      tag: scriptDraft,
    });
    setScriptData({ tag: scriptDraft });
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
                            checked={cameraDraft?.active ?? cameraData.active}
                            onChange={(e) =>
                              handleCameraChange({
                                ...(cameraDraft ?? cameraData),
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
                            value={cameraDraft?.zoom ?? cameraData.zoom}
                            onChange={(e) =>
                              setCameraDraft((prev) => ({
                                ...(prev ?? cameraData),
                                zoom: parseFloat(e.target.value) || 0.01,
                              }))
                            }
                            onBlur={() =>
                              cameraDraft && handleCameraChange(cameraDraft)
                            }
                            onKeyDown={(e) => {
                              if (e.key === "Enter") {
                                (e.target as HTMLInputElement).blur();
                              }
                            }}
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
                            value={cameraDraft?.rotation ?? cameraData.rotation}
                            onChange={(e) =>
                              setCameraDraft((prev) => ({
                                ...(prev ?? cameraData),
                                rotation: parseFloat(e.target.value) || 0,
                              }))
                            }
                            onBlur={() =>
                              cameraDraft && handleCameraChange(cameraDraft)
                            }
                            onKeyDown={(e) => {
                              if (e.key === "Enter") {
                                (e.target as HTMLInputElement).blur();
                              }
                            }}
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
                              value={cameraDraft?.offset[0] ?? cameraData.offset[0]}
                              onChange={(e) =>
                                setCameraDraft((prev) => ({
                                  ...(prev ?? cameraData),
                                  offset: [
                                    parseFloat(e.target.value) || 0,
                                    (prev ?? cameraData).offset[1],
                                  ],
                                }))
                              }
                              onBlur={() =>
                                cameraDraft && handleCameraChange(cameraDraft)
                              }
                              onKeyDown={(e) => {
                                if (e.key === "Enter") {
                                  (e.target as HTMLInputElement).blur();
                                }
                              }}
                              className="px-2 py-1 bg-gray-700 rounded text-sm"
                            />
                            <input
                              type="number"
                              step="1"
                              value={cameraDraft?.offset[1] ?? cameraData.offset[1]}
                              onChange={(e) =>
                                setCameraDraft((prev) => ({
                                  ...(prev ?? cameraData),
                                  offset: [
                                    (prev ?? cameraData).offset[0],
                                    parseFloat(e.target.value) || 0,
                                  ],
                                }))
                              }
                              onBlur={() =>
                                cameraDraft && handleCameraChange(cameraDraft)
                              }
                              onKeyDown={(e) => {
                                if (e.key === "Enter") {
                                  (e.target as HTMLInputElement).blur();
                                }
                              }}
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
                {type === "ScriptTag" && (
                  <div className="rounded border border-gray-700/60 bg-gray-800/40 p-2">
                    <div className="text-xs text-gray-400 mb-2">Script</div>
                    <div className="flex gap-2 mb-2">
                      <button
                        onClick={handlePickScript}
                        className="px-2 py-1 rounded bg-blue-600/80 text-xs"
                      >
                        Pick
                      </button>
                      <div className="flex-1 truncate text-xs text-gray-400">
                        {scriptData?.tag || "No script set"}
                      </div>
                    </div>
                    <input
                      type="text"
                      value={scriptDraft}
                      onChange={(e) => setScriptDraft(e.target.value)}
                      onBlur={commitScriptDraft}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          (e.target as HTMLInputElement).blur();
                        }
                      }}
                      className="w-full px-2 py-1 bg-gray-700 rounded text-sm"
                      placeholder="Path to script file"
                    />
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
                          value={(fieldDrafts[fieldKey(type, field.name)] as string) ?? String(field.value ?? 0)}
                          onChange={(e) =>
                            setFieldDrafts((prev) => ({
                              ...prev,
                              [fieldKey(type, field.name)]: e.target.value,
                            }))
                          }
                          onBlur={() => commitFieldDraft(type, field)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              (e.target as HTMLInputElement).blur();
                            }
                          }}
                          className="w-full px-2 py-1 bg-gray-700 rounded text-sm"
                        />
                      ) : field.type_name === "Vec2" ? (
                        <div className="grid grid-cols-2 gap-2">
                          {(() => {
                            const draft = fieldDrafts[fieldKey(type, field.name)];
                            const vecDraft =
                              typeof draft === "string" || !draft
                                ? {
                                    x: String((field.value as any)?.x ?? 0),
                                    y: String((field.value as any)?.y ?? 0),
                                  }
                                : draft;
                            return (
                              <>
                          <input
                            type="number"
                            step="0.1"
                            value={vecDraft.x}
                            onChange={(e) =>
                              setFieldDrafts((prev) => ({
                                ...prev,
                                [fieldKey(type, field.name)]: {
                                  ...vecDraft,
                                  x: e.target.value,
                                },
                              }))
                            }
                            onBlur={() => commitFieldDraft(type, field)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") {
                                (e.target as HTMLInputElement).blur();
                              }
                            }}
                            placeholder="X"
                            className="px-2 py-1 bg-gray-700 rounded text-sm"
                          />
                          <input
                            type="number"
                            step="0.1"
                            value={vecDraft.y}
                            onChange={(e) =>
                              setFieldDrafts((prev) => ({
                                ...prev,
                                [fieldKey(type, field.name)]: {
                                  ...vecDraft,
                                  y: e.target.value,
                                },
                              }))
                            }
                            onBlur={() => commitFieldDraft(type, field)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") {
                                (e.target as HTMLInputElement).blur();
                              }
                            }}
                            placeholder="Y"
                            className="px-2 py-1 bg-gray-700 rounded text-sm"
                          />
                              </>
                            );
                          })()}
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
