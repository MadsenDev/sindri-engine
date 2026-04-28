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
  refreshTrigger?: number;
}

export default function Inspector({ selectedEntityId, refreshTrigger }: InspectorProps) {
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

    const load = async () => {
      const snapshot = await invoke<InspectorSnapshot>("inspector_snapshot", {
        entityId: selectedEntityId,
      });
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

    load().catch((e) => console.error("Failed to load inspector snapshot", e));
  }, [selectedEntityId, refreshTrigger]);

  const handleFieldChange = async (componentType: string, fieldName: string, value: any) => {
    if (selectedEntityId === null) return;
    await invoke("component_set_field", { entityId: selectedEntityId, componentType, fieldName, value });
    const componentFields = await invoke<ComponentFieldInfo[] | null>("component_fields", {
      entityId: selectedEntityId,
      componentType,
    });
    if (componentFields) {
      setFields((prev) => ({ ...prev, [componentType]: componentFields }));
      const draftKey = fieldKey(componentType, fieldName);
      const updated = componentFields.find((f) => f.name === fieldName);
      if (updated) {
        setFieldDrafts((prev) => ({
          ...prev,
          [draftKey]:
            updated.type_name === "Vec2"
              ? { x: String((updated.value as any)?.x ?? 0), y: String((updated.value as any)?.y ?? 0) }
              : String(updated.value ?? 0),
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
    } else if (field.type_name === "Vec2" && typeof draft !== "string") {
      await handleFieldChange(componentType, field.name, {
        x: parseFloat(draft.x) || 0,
        y: parseFloat(draft.y) || 0,
      });
    }
  };

  const handleAddComponent = async () => {
    if (selectedEntityId === null || !componentToAdd) return;
    await invoke("component_add", { entityId: selectedEntityId, componentType: componentToAdd });
    const attached = await invoke<string[]>("entity_components", { entityId: selectedEntityId });
    setAttachedComponents(attached);
    if (componentToAdd === "SpriteComponent") {
      const sprite = await invoke<SpriteData | null>("sprite_get", { entityId: selectedEntityId });
      setSpriteData(sprite);
    }
    if (componentToAdd === "CameraComponent") {
      const cam = await invoke<CameraData | null>("camera_get", { entityId: selectedEntityId });
      setCameraData(cam);
      setCameraDraft(cam);
    }
    if (componentToAdd === "ScriptTag") {
      const script = await invoke<ScriptTagData | null>("script_tag_get", { entityId: selectedEntityId });
      setScriptData(script);
      setScriptDraft(script?.tag ?? "");
    }
  };

  const handleRemoveComponent = async (componentType: string) => {
    if (selectedEntityId === null) return;
    await invoke("component_remove", { entityId: selectedEntityId, componentType });
    const attached = await invoke<string[]>("entity_components", { entityId: selectedEntityId });
    setAttachedComponents(attached);
    setFields((prev) => {
      const next = { ...prev };
      delete next[componentType];
      return next;
    });
    if (componentType === "SpriteComponent") setSpriteData(null);
    if (componentType === "CameraComponent") { setCameraData(null); setCameraDraft(null); }
    if (componentType === "ScriptTag") { setScriptData(null); setScriptDraft(""); }
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
    const cam = await invoke<CameraData | null>("camera_get", { entityId: selectedEntityId });
    setCameraData(cam);
    setCameraDraft(cam);
  };

  const handlePickTexture = async () => {
    if (selectedEntityId === null) return;
    const filePath = await open({ filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "gif", "webp"] }] });
    if (!filePath || typeof filePath !== "string") return;
    let importedPath = filePath;
    try {
      importedPath = await invoke<string>("asset_import_texture", { path: filePath });
    } catch (e) {
      console.error("Texture import failed:", e);
    }
    await invoke("sprite_set_texture_path", { entityId: selectedEntityId, path: importedPath });
    const sprite = await invoke<SpriteData | null>("sprite_get", { entityId: selectedEntityId });
    setSpriteData(sprite);
  };

  const handleClearTexture = async () => {
    if (selectedEntityId === null) return;
    await invoke("sprite_set_texture_path", { entityId: selectedEntityId, path: "" });
    const sprite = await invoke<SpriteData | null>("sprite_get", { entityId: selectedEntityId });
    setSpriteData(sprite);
  };

  const handlePickScript = async () => {
    if (selectedEntityId === null) return;
    const filePath = await open({ filters: [{ name: "Script", extensions: ["lua", "rhai"] }] });
    if (!filePath || typeof filePath !== "string") return;
    await invoke("script_tag_set", { entityId: selectedEntityId, tag: filePath });
    const script = await invoke<ScriptTagData | null>("script_tag_get", { entityId: selectedEntityId });
    setScriptData(script);
    setScriptDraft(script?.tag ?? "");
  };

  const commitScriptDraft = async () => {
    if (selectedEntityId === null) return;
    await invoke("script_tag_set", { entityId: selectedEntityId, tag: scriptDraft });
    setScriptData({ tag: scriptDraft });
  };

  if (selectedEntityId === null) {
    return <div className="inspector-empty">Select an entity to inspect</div>;
  }

  return (
    <div className="inspector-scroll">
      {attachedComponents.length === 0 ? (
        <div className="inspector-empty">No components</div>
      ) : (
        attachedComponents.map((type) => {
          const componentFields = fields[type];

          return (
            <div key={type} className="inspector-section">
              <div className="inspector-section-header">
                <span className="inspector-component-name">{type}</span>
                <button
                  onClick={() => handleRemoveComponent(type)}
                  className="unity-button danger small"
                >
                  Remove
                </button>
              </div>

              <div className="inspector-fields">
                {type === "SpriteComponent" && (
                  <div className="inspector-special">
                    <div className="inspector-field-label">Texture</div>
                    <div className="panel-actions">
                      <button onClick={handlePickTexture} className="unity-button muted small">Pick</button>
                      <button onClick={handleClearTexture} className="unity-button muted small">Clear</button>
                      <span className="panel-footnote" style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {spriteData?.texture_path ?? "None"}
                      </span>
                    </div>
                    {spriteData?.texture_path && (
                      <img
                        src={spriteData.texture_path}
                        alt="texture"
                        className="inspector-texture-preview"
                      />
                    )}
                  </div>
                )}

                {type === "CameraComponent" && (
                  <div className="inspector-special">
                    <div className="inspector-field-label">Camera</div>
                    {cameraData ? (
                      <>
                        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, color: "var(--unity-muted)", cursor: "pointer" }}>
                          <input
                            type="checkbox"
                            checked={cameraDraft?.active ?? cameraData.active}
                            onChange={(e) =>
                              handleCameraChange({ ...(cameraDraft ?? cameraData), active: e.target.checked })
                            }
                          />
                          Active
                        </label>
                        <div className="inspector-field">
                          <span className="inspector-field-label">Zoom</span>
                          <input
                            type="number"
                            step="0.05"
                            value={cameraDraft?.zoom ?? cameraData.zoom}
                            onChange={(e) => setCameraDraft((prev) => ({ ...(prev ?? cameraData), zoom: parseFloat(e.target.value) || 0.01 }))}
                            onBlur={() => cameraDraft && handleCameraChange(cameraDraft)}
                            onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                            className="inspector-input"
                          />
                        </div>
                        <div className="inspector-field">
                          <span className="inspector-field-label">Rotation</span>
                          <input
                            type="number"
                            step="0.05"
                            value={cameraDraft?.rotation ?? cameraData.rotation}
                            onChange={(e) => setCameraDraft((prev) => ({ ...(prev ?? cameraData), rotation: parseFloat(e.target.value) || 0 }))}
                            onBlur={() => cameraDraft && handleCameraChange(cameraDraft)}
                            onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                            className="inspector-input"
                          />
                        </div>
                        <div className="inspector-field">
                          <span className="inspector-field-label">Offset</span>
                          <div className="inspector-input-row">
                            <input
                              type="number"
                              step="1"
                              value={cameraDraft?.offset[0] ?? cameraData.offset[0]}
                              onChange={(e) => setCameraDraft((prev) => ({ ...(prev ?? cameraData), offset: [parseFloat(e.target.value) || 0, (prev ?? cameraData).offset[1]] }))}
                              onBlur={() => cameraDraft && handleCameraChange(cameraDraft)}
                              onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                              className="inspector-input"
                              placeholder="X"
                            />
                            <input
                              type="number"
                              step="1"
                              value={cameraDraft?.offset[1] ?? cameraData.offset[1]}
                              onChange={(e) => setCameraDraft((prev) => ({ ...(prev ?? cameraData), offset: [(prev ?? cameraData).offset[0], parseFloat(e.target.value) || 0] }))}
                              onBlur={() => cameraDraft && handleCameraChange(cameraDraft)}
                              onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                              className="inspector-input"
                              placeholder="Y"
                            />
                          </div>
                        </div>
                      </>
                    ) : (
                      <div className="inspector-field-label">No camera data</div>
                    )}
                  </div>
                )}

                {type === "ScriptTag" && (
                  <div className="inspector-special">
                    <div className="inspector-field-label">Script</div>
                    <div className="panel-actions">
                      <button onClick={handlePickScript} className="unity-button muted small">Pick</button>
                      <span className="panel-footnote" style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {scriptData?.tag || "None"}
                      </span>
                    </div>
                    <input
                      type="text"
                      value={scriptDraft}
                      onChange={(e) => setScriptDraft(e.target.value)}
                      onBlur={commitScriptDraft}
                      onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                      className="inspector-input"
                      placeholder="Path to script file"
                    />
                  </div>
                )}

                {componentFields && componentFields.length > 0 && componentFields.map((field) => (
                  <div key={field.name} className="inspector-field">
                    <span className="inspector-field-label">
                      {field.name}{" "}
                      <span style={{ opacity: 0.45 }}>({field.type_name})</span>
                    </span>
                    {field.type_name === "f32" ? (
                      <input
                        type="number"
                        step="0.1"
                        value={(fieldDrafts[fieldKey(type, field.name)] as string) ?? String(field.value ?? 0)}
                        onChange={(e) => setFieldDrafts((prev) => ({ ...prev, [fieldKey(type, field.name)]: e.target.value }))}
                        onBlur={() => commitFieldDraft(type, field)}
                        onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                        className="inspector-input"
                      />
                    ) : field.type_name === "Vec2" ? (
                      (() => {
                        const draft = fieldDrafts[fieldKey(type, field.name)];
                        const vecDraft =
                          typeof draft === "string" || !draft
                            ? { x: String((field.value as any)?.x ?? 0), y: String((field.value as any)?.y ?? 0) }
                            : draft;
                        return (
                          <div className="inspector-input-row">
                            <input
                              type="number"
                              step="0.1"
                              value={vecDraft.x}
                              onChange={(e) => setFieldDrafts((prev) => ({ ...prev, [fieldKey(type, field.name)]: { ...vecDraft, x: e.target.value } }))}
                              onBlur={() => commitFieldDraft(type, field)}
                              onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                              placeholder="X"
                              className="inspector-input"
                            />
                            <input
                              type="number"
                              step="0.1"
                              value={vecDraft.y}
                              onChange={(e) => setFieldDrafts((prev) => ({ ...prev, [fieldKey(type, field.name)]: { ...vecDraft, y: e.target.value } }))}
                              onBlur={() => commitFieldDraft(type, field)}
                              onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                              placeholder="Y"
                              className="inspector-input"
                            />
                          </div>
                        );
                      })()
                    ) : (
                      <div className="inspector-input" style={{ opacity: 0.6, userSelect: "none" }}>
                        {JSON.stringify(field.value)}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          );
        })
      )}

      <div className="inspector-add-section">
        <div className="inspector-add-title">Add Component</div>
        <div className="inspector-add-controls">
          <select
            value={componentToAdd}
            onChange={(e) => setComponentToAdd(e.target.value)}
            className="inspector-select"
          >
            {attachableTypes.map((type) => (
              <option key={type} value={type}>{type}</option>
            ))}
          </select>
          <button onClick={handleAddComponent} className="unity-button primary">
            Add
          </button>
        </div>
      </div>
    </div>
  );
}
