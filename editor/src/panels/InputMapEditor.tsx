import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

type BindingEntry =
  | { type: "key"; key: string }
  | { type: "key_axis"; negative: string; positive: string }
  | { type: "gamepad_button"; button: string }
  | { type: "gamepad_axis"; axis: string; deadzone: number };

interface ActionEntry {
  name: string;
  bindings: BindingEntry[];
}

interface InputMapConfig {
  actions: ActionEntry[];
}

const GAMEPAD_BUTTONS = [
  "South", "East", "North", "West",
  "LeftTrigger", "RightTrigger", "LeftThumb", "RightThumb",
  "Start", "Select", "DPadUp", "DPadDown", "DPadLeft", "DPadRight",
];

const GAMEPAD_AXES = [
  "LeftStickX", "LeftStickY", "RightStickX", "RightStickY",
  "LeftZ", "RightZ",
];

function CustomSelect({
  value,
  options,
  onChange,
  style,
}: {
  value: string;
  options: string[];
  onChange: (v: string) => void;
  style?: React.CSSProperties;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div ref={ref} style={{ position: "relative", flex: 1, minWidth: 0, ...style }}>
      <div
        onClick={() => setOpen(o => !o)}
        style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          fontSize: "11px", fontFamily: "var(--font-mono)",
          padding: "3px 6px", background: "var(--paper-2)",
          border: "1px solid var(--rule)", borderRadius: "4px",
          color: "var(--ink)", cursor: "pointer", userSelect: "none",
          gap: "6px",
        }}
      >
        <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {value}
        </span>
        <span style={{ color: "var(--ink-3)", fontSize: "9px", flexShrink: 0 }}>▼</span>
      </div>
      {open && (
        <div style={{
          position: "absolute", top: "calc(100% + 2px)", left: 0, right: 0,
          background: "var(--paper)", border: "1px solid var(--rule)",
          borderRadius: "4px", zIndex: 999, overflow: "hidden",
          boxShadow: "0 4px 12px rgba(0,0,0,0.4)",
          maxHeight: "180px", overflowY: "auto",
        }}>
          {options.map(opt => (
            <div
              key={opt}
              onMouseDown={() => { onChange(opt); setOpen(false); }}
              style={{
                padding: "5px 8px", fontSize: "11px",
                fontFamily: "var(--font-mono)", cursor: "pointer",
                color: opt === value ? "var(--ink)" : "var(--ink-2)",
                background: opt === value ? "var(--paper-2)" : "transparent",
              }}
              onMouseEnter={e => (e.currentTarget.style.background = "var(--paper-2)")}
              onMouseLeave={e => (e.currentTarget.style.background = opt === value ? "var(--paper-2)" : "transparent")}
            >
              {opt}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function BindingTag({
  binding,
  onRemove,
}: {
  binding: BindingEntry;
  onRemove: () => void;
}) {
  let label = "";
  if (binding.type === "key") label = `Key: ${binding.key}`;
  else if (binding.type === "key_axis") label = `Axis: ${binding.negative} / ${binding.positive}`;
  else if (binding.type === "gamepad_button") label = `Btn: ${binding.button}`;
  else if (binding.type === "gamepad_axis") label = `Axis: ${binding.axis}`;

  return (
    <span style={{
      display: "inline-flex", alignItems: "center", gap: "4px",
      background: "var(--paper-2)", border: "1px solid var(--rule)",
      borderRadius: "4px", padding: "2px 6px", fontSize: "11px",
      fontFamily: "var(--font-mono)", color: "var(--ink-2)",
    }}>
      {label}
      <button
        onClick={onRemove}
        style={{
          background: "none", border: "none", padding: "0 2px",
          cursor: "pointer", color: "var(--ink-3)", fontSize: "11px",
          lineHeight: 1,
        }}
      >×</button>
    </span>
  );
}

type AddBindingState =
  | { type: "key"; key: string }
  | { type: "key_axis"; negative: string; positive: string }
  | { type: "gamepad_button"; button: string }
  | { type: "gamepad_axis"; axis: string; deadzone: number };

function AddBindingForm({ onAdd }: { onAdd: (b: BindingEntry) => void }) {
  const [kind, setKind] = useState<"key" | "key_axis" | "gamepad_button" | "gamepad_axis">("key");
  const [draft, setDraft] = useState<AddBindingState>({ type: "key", key: "" });
  const [capturing, setCapturing] = useState(false);

  useEffect(() => {
    if (kind === "key") setDraft({ type: "key", key: "" });
    else if (kind === "key_axis") setDraft({ type: "key_axis", negative: "", positive: "" });
    else if (kind === "gamepad_button") setDraft({ type: "gamepad_button", button: GAMEPAD_BUTTONS[0] });
    else if (kind === "gamepad_axis") setDraft({ type: "gamepad_axis", axis: GAMEPAD_AXES[0], deadzone: 0.2 });
  }, [kind]);

  const handleKeyCapture = useCallback((e: KeyboardEvent) => {
    if (!capturing) return;
    e.preventDefault();
    const key = e.key === " " ? " " : e.key;
    if (draft.type === "key") setDraft({ type: "key", key });
    setCapturing(false);
  }, [capturing, draft.type]);

  useEffect(() => {
    if (capturing) {
      window.addEventListener("keydown", handleKeyCapture);
      return () => window.removeEventListener("keydown", handleKeyCapture);
    }
  }, [capturing, handleKeyCapture]);

  const canSubmit = () => {
    if (draft.type === "key") return draft.key.length > 0;
    if (draft.type === "key_axis") return draft.negative.length > 0 && draft.positive.length > 0;
    return true;
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "6px", padding: "6px 0" }}>
      <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
        {(["key", "key_axis", "gamepad_button", "gamepad_axis"] as const).map(k => (
          <button key={k} onClick={() => setKind(k)} style={{
            fontSize: "11px", padding: "2px 8px",
            background: kind === k ? "var(--accent)" : "var(--paper-2)",
            color: kind === k ? "#fff" : "var(--ink-2)",
            border: "1px solid var(--rule)", borderRadius: "4px", cursor: "pointer",
          }}>
            {k === "key" ? "Key" : k === "key_axis" ? "Key Axis" : k === "gamepad_button" ? "Btn" : "GP Axis"}
          </button>
        ))}
      </div>

      {draft.type === "key" && (
        <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
          <input
            value={draft.key}
            onChange={e => setDraft({ type: "key", key: e.target.value })}
            placeholder="key name"
            style={inputStyle}
            readOnly={capturing}
          />
          <button onClick={() => setCapturing(true)} style={smallBtnStyle}>
            {capturing ? "Press…" : "Capture"}
          </button>
        </div>
      )}

      {draft.type === "key_axis" && (
        <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
          <input
            value={draft.negative}
            onChange={e => setDraft({ ...draft, negative: e.target.value })}
            placeholder="negative key"
            style={inputStyle}
          />
          <span style={{ color: "var(--ink-3)", fontSize: "11px" }}>/</span>
          <input
            value={draft.positive}
            onChange={e => setDraft({ ...draft, positive: e.target.value })}
            placeholder="positive key"
            style={inputStyle}
          />
        </div>
      )}

      {draft.type === "gamepad_button" && (
        <CustomSelect
          value={draft.button}
          options={GAMEPAD_BUTTONS}
          onChange={v => setDraft({ type: "gamepad_button", button: v })}
        />
      )}

      {draft.type === "gamepad_axis" && (
        <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
          <CustomSelect
            value={draft.axis}
            options={GAMEPAD_AXES}
            onChange={v => setDraft({ ...draft, axis: v })}
          />
          <span style={{ color: "var(--ink-3)", fontSize: "11px" }}>dz</span>
          <input
            type="number"
            step="0.05"
            min="0"
            max="1"
            value={draft.type === "gamepad_axis" ? draft.deadzone : 0.2}
            onChange={e => setDraft({ ...draft, deadzone: parseFloat(e.target.value) || 0.2 })}
            style={{ ...inputStyle, width: "52px" }}
          />
        </div>
      )}

      <button
        onClick={() => { if (canSubmit()) { onAdd(draft as BindingEntry); } }}
        disabled={!canSubmit()}
        style={{
          ...smallBtnStyle,
          alignSelf: "flex-start",
          opacity: canSubmit() ? 1 : 0.4,
        }}
      >
        + Add binding
      </button>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  fontSize: "11px",
  fontFamily: "var(--font-mono)",
  padding: "3px 6px",
  background: "var(--paper-2)",
  border: "1px solid var(--rule)",
  borderRadius: "4px",
  color: "var(--ink)",
  outline: "none",
  flex: 1,
  minWidth: 0,
};

const smallBtnStyle: React.CSSProperties = {
  fontSize: "11px",
  padding: "3px 8px",
  background: "var(--paper-2)",
  border: "1px solid var(--rule)",
  borderRadius: "4px",
  cursor: "pointer",
  color: "var(--ink-2)",
};

interface Props {
  projectPath: string | null;
}

export default function InputMapEditor({ projectPath }: Props) {
  const [config, setConfig] = useState<InputMapConfig>({ actions: [] });
  const [expandedIdx, setExpandedIdx] = useState<number | null>(null);
  const [newActionName, setNewActionName] = useState("");
  const [dirty, setDirty] = useState(false);

  const load = useCallback(async () => {
    if (!projectPath) return;
    try {
      const data = await invoke<InputMapConfig>("read_input_map", { projectPath });
      setConfig(data);
      setDirty(false);
    } catch (e) {
      console.error("Failed to load input map:", e);
    }
  }, [projectPath]);

  useEffect(() => { load(); }, [load]);

  const save = async () => {
    if (!projectPath) return;
    await invoke("write_input_map", { projectPath, data: config });
    setDirty(false);
  };

  const update = (newConfig: InputMapConfig) => {
    setConfig(newConfig);
    setDirty(true);
  };

  const addAction = () => {
    const name = newActionName.trim();
    if (!name) return;
    if (config.actions.some(a => a.name === name)) return;
    const newActions = [...config.actions, { name, bindings: [] }];
    update({ ...config, actions: newActions });
    setNewActionName("");
    setExpandedIdx(newActions.length - 1);
  };

  const removeAction = (idx: number) => {
    const newActions = config.actions.filter((_, i) => i !== idx);
    update({ ...config, actions: newActions });
    if (expandedIdx === idx) setExpandedIdx(null);
  };

  const addBinding = (actionIdx: number, binding: BindingEntry) => {
    const newActions = config.actions.map((a, i) =>
      i === actionIdx ? { ...a, bindings: [...a.bindings, binding] } : a
    );
    update({ ...config, actions: newActions });
  };

  const removeBinding = (actionIdx: number, bindingIdx: number) => {
    const newActions = config.actions.map((a, i) =>
      i === actionIdx
        ? { ...a, bindings: a.bindings.filter((_, j) => j !== bindingIdx) }
        : a
    );
    update({ ...config, actions: newActions });
  };

  return (
    <div style={{
      flex: 1, overflow: "hidden", display: "flex", flexDirection: "column",
      fontFamily: "var(--font-ui)", fontSize: "12px",
    }}>
      {/* Toolbar */}
      <div style={{
        display: "flex", alignItems: "center", gap: "8px",
        padding: "8px 12px", borderBottom: "1px solid var(--rule)", flexShrink: 0,
      }}>
        <span style={{ color: "var(--ink-2)", fontSize: "11px", flex: 1 }}>Input Map</span>
        {dirty && (
          <button onClick={save} style={{
            fontSize: "11px", padding: "3px 10px",
            background: "var(--accent)", color: "#fff",
            border: "none", borderRadius: "4px", cursor: "pointer",
          }}>Save</button>
        )}
      </div>

      {/* Action list */}
      <div style={{ flex: 1, overflow: "auto", padding: "8px" }}>
        {config.actions.length === 0 && (
          <div style={{ color: "var(--ink-3)", fontSize: "11px", padding: "8px 4px" }}>
            No actions yet. Add one below.
          </div>
        )}

        {config.actions.map((action, idx) => (
          <div key={action.name} style={{
            border: "1px solid var(--rule)", borderRadius: "6px",
            marginBottom: "6px", overflow: "hidden",
          }}>
            {/* Action header */}
            <div
              onClick={() => setExpandedIdx(expandedIdx === idx ? null : idx)}
              style={{
                display: "flex", alignItems: "center", gap: "8px",
                padding: "7px 10px", cursor: "pointer",
                background: expandedIdx === idx ? "var(--paper-2)" : "transparent",
                userSelect: "none",
              }}
            >
              <span style={{ fontSize: "10px", color: "var(--ink-3)" }}>
                {expandedIdx === idx ? "▾" : "▸"}
              </span>
              <span style={{ flex: 1, fontFamily: "var(--font-mono)", fontWeight: 500 }}>
                {action.name}
              </span>
              <span style={{ fontSize: "11px", color: "var(--ink-3)" }}>
                {action.bindings.length} binding{action.bindings.length !== 1 ? "s" : ""}
              </span>
              <button
                onClick={e => { e.stopPropagation(); removeAction(idx); }}
                style={{
                  background: "none", border: "none", cursor: "pointer",
                  color: "var(--ink-3)", fontSize: "14px", padding: "0 2px", lineHeight: 1,
                }}
              >×</button>
            </div>

            {/* Expanded content */}
            {expandedIdx === idx && (
              <div style={{ padding: "8px 12px", borderTop: "1px solid var(--rule)" }}>
                {/* Existing bindings */}
                <div style={{ display: "flex", flexWrap: "wrap", gap: "4px", marginBottom: "8px" }}>
                  {action.bindings.map((b, bi) => (
                    <BindingTag key={bi} binding={b} onRemove={() => removeBinding(idx, bi)} />
                  ))}
                  {action.bindings.length === 0 && (
                    <span style={{ color: "var(--ink-3)", fontSize: "11px" }}>No bindings</span>
                  )}
                </div>
                <AddBindingForm onAdd={b => addBinding(idx, b)} />
              </div>
            )}
          </div>
        ))}
      </div>

      {/* New action row */}
      <div style={{
        display: "flex", gap: "6px", padding: "8px 12px",
        borderTop: "1px solid var(--rule)", flexShrink: 0,
      }}>
        <input
          value={newActionName}
          onChange={e => setNewActionName(e.target.value)}
          onKeyDown={e => { if (e.key === "Enter") addAction(); }}
          placeholder="action name"
          style={{ ...inputStyle, flex: 1 }}
        />
        <button onClick={addAction} style={smallBtnStyle}>+ Action</button>
      </div>
    </div>
  );
}
