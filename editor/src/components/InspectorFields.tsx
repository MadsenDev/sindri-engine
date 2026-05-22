import { useState, useRef, useEffect, type CSSProperties, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useComponentPatch(entityId: number, componentIdx: number, onSceneChange: () => void) {
  return async (data: Record<string, unknown>) => {
    try {
      await invoke("patch_component", { entityId, componentIdx, data });
      onSceneChange();
    } catch (err) {
      console.error("patch_component failed:", err);
    }
  };
}

export function EditableRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ display: "flex", alignItems: "center", height: "26px", padding: "0 22px", gap: "8px" }}>
      <span style={{ width: "78px", fontSize: "12px", color: "var(--ink-3)", flexShrink: 0 }}>{label}</span>
      {children}
    </div>
  );
}

export function inputStyle(focused: boolean): CSSProperties {
  return {
    flex: 1, minWidth: 0,
    height: "20px",
    background: "var(--paper-2)",
    border: `1px solid ${focused ? "var(--amber)" : "var(--rule-2)"}`,
    color: "var(--ink)",
    fontFamily: "var(--font-mono)",
    fontSize: "12px",
    padding: "0 6px",
    outline: "none",
  };
}

export function TextInputField({ label, value, placeholder, onCommit }: {
  label: string; value: string; placeholder?: string; onCommit: (value: string) => void;
}) {
  const [focused, setFocused] = useState(false);
  return (
    <EditableRow label={label}>
      <input
        defaultValue={value} placeholder={placeholder}
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); onCommit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

export function NumberInputField({ label, value, decimals = 2, min, max, onCommit }: {
  label: string; value: number; decimals?: number; min?: number; max?: number; onCommit: (value: number) => void;
}) {
  const [focused, setFocused] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const commit = (raw: string) => {
    if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; }
    let next = Number(raw);
    if (!Number.isFinite(next)) return;
    if (min !== undefined) next = Math.max(min, next);
    if (max !== undefined) next = Math.min(max, next);
    onCommit(next);
  };

  const handleChange = (raw: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => { commit(raw); }, 280);
  };

  return (
    <EditableRow label={label}>
      <input
        type="number"
        defaultValue={decimals === 0 ? String(Math.round(value)) : value.toFixed(decimals)}
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); commit(e.currentTarget.value); }}
        onChange={e => handleChange(e.currentTarget.value)}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

export function OptionalEntityField({ label, value, onCommit }: {
  label: string; value: number | null; onCommit: (value: number | null) => void;
}) {
  const [focused, setFocused] = useState(false);
  const commit = (raw: string) => {
    const trimmed = raw.trim();
    if (trimmed === "") { onCommit(null); return; }
    const next = Number(trimmed);
    if (Number.isInteger(next) && next >= 0) onCommit(next);
  };
  return (
    <EditableRow label={label}>
      <input
        type="number" defaultValue={value ?? ""} placeholder="none"
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); commit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

export function OptionalNumberField({ label, value, onCommit }: {
  label: string; value: number | null; onCommit: (value: number | null) => void;
}) {
  const [focused, setFocused] = useState(false);
  const commit = (raw: string) => {
    const trimmed = raw.trim();
    if (trimmed === "") { onCommit(null); return; }
    const next = Number(trimmed);
    if (Number.isFinite(next)) onCommit(next);
  };
  return (
    <EditableRow label={label}>
      <input
        type="number" defaultValue={value ?? ""} placeholder="none"
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); commit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </EditableRow>
  );
}

export function BoolField({ label, value, onChange }: { label: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <label style={{ display: "flex", alignItems: "center", height: "24px", padding: "0 22px", gap: "8px", cursor: "pointer" }}>
      <span style={{ width: "78px", fontSize: "12px", color: "var(--ink-3)", flexShrink: 0 }}>{label}</span>
      <input type="checkbox" checked={value} onChange={e => onChange(e.currentTarget.checked)} />
      <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: value ? "var(--amber)" : "var(--ink-4)" }}>
        {String(value)}
      </span>
    </label>
  );
}

export function ColorField({ label, value, onCommit }: {
  label: string; value: [number, number, number, number]; onCommit: (value: [number, number, number, number]) => void;
}) {
  const [focused, setFocused] = useState<number | null>(null);
  const pickerRef = useRef<HTMLInputElement>(null);

  const toHex = (c: [number, number, number, number]) =>
    "#" + [c[0], c[1], c[2]].map(v => Math.round(v * 255).toString(16).padStart(2, "0")).join("");

  const commit = (idx: number, raw: string) => {
    const next = Number(raw);
    if (!Number.isFinite(next)) return;
    const color: [number, number, number, number] = [...value] as [number, number, number, number];
    color[idx] = Math.max(0, Math.min(1, next));
    onCommit(color);
  };

  const handlePickerChange = (hex: string) => {
    const r = parseInt(hex.slice(1, 3), 16) / 255;
    const g = parseInt(hex.slice(3, 5), 16) / 255;
    const b = parseInt(hex.slice(5, 7), 16) / 255;
    onCommit([r, g, b, value[3]]);
  };

  const swatchBg = `rgba(${Math.round(value[0]*255)},${Math.round(value[1]*255)},${Math.round(value[2]*255)},${value[3]})`;

  return (
    <EditableRow label={label}>
      <div
        onClick={() => pickerRef.current?.click()}
        title="Open color picker"
        style={{
          width: "20px", height: "20px", flexShrink: 0,
          background: swatchBg, border: "1px solid var(--rule-2)",
          cursor: "pointer",
        }}
      />
      <input
        ref={pickerRef}
        type="color"
        value={toHex(value)}
        onChange={e => handlePickerChange(e.target.value)}
        style={{ position: "absolute", opacity: 0, pointerEvents: "none", width: 0, height: 0 }}
      />
      {value.map((channel, idx) => (
        <input
          key={idx} type="number" step="0.01" min="0" max="1"
          defaultValue={channel.toFixed(2)}
          onFocus={() => setFocused(idx)}
          onBlur={e => { setFocused(null); commit(idx, e.currentTarget.value); }}
          onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
          style={inputStyle(focused === idx)}
        />
      ))}
    </EditableRow>
  );
}

export function BrowseInputField({ label, value, placeholder, onCommit, onBrowse }: {
  label: string; value: string; placeholder?: string; onCommit: (value: string) => void; onBrowse: () => void;
}) {
  const [focused, setFocused] = useState(false);
  return (
    <EditableRow label={label}>
      <input
        defaultValue={value} placeholder={placeholder}
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); onCommit(e.currentTarget.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={{ ...inputStyle(focused), flex: 1 }}
      />
      <button
        onClick={onBrowse}
        title="Browse"
        style={{
          flexShrink: 0, height: "20px", padding: "0 6px",
          background: "var(--paper-2)", border: "1px solid var(--rule-2)",
          color: "var(--ink-3)", fontFamily: "var(--font-ui)", fontSize: "11px",
          cursor: "pointer",
        }}
        onMouseEnter={e => (e.currentTarget as HTMLElement).style.color = "var(--ink)"}
        onMouseLeave={e => (e.currentTarget as HTMLElement).style.color = "var(--ink-3)"}
      >
        ···
      </button>
    </EditableRow>
  );
}

export function CustomSelect({ value, options, onChange }: {
  value: string; options: string[]; onChange: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", handler);
    return () => window.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div ref={ref} style={{ flex: 1, position: "relative" }}>
      <button
        onClick={() => setOpen(o => !o)}
        style={{
          width: "100%", height: "20px",
          background: "var(--paper-2)", border: "1px solid var(--rule-2)",
          color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px",
          padding: "0 6px", cursor: "pointer",
          display: "flex", alignItems: "center", justifyContent: "space-between",
        }}
      >
        <span>{value}</span>
        <span style={{ color: "var(--ink-4)", fontSize: "9px", marginLeft: "4px" }}>▾</span>
      </button>
      {open && (
        <div style={{
          position: "absolute", top: "100%", left: 0, right: 0, zIndex: 200,
          background: "var(--paper)", border: "1px solid var(--rule-2)",
          boxShadow: "0 4px 16px rgba(0,0,0,0.4)",
        }}>
          {options.map(opt => (
            <button
              key={opt}
              onClick={() => { onChange(opt); setOpen(false); }}
              style={{
                display: "block", width: "100%", padding: "6px 8px",
                background: opt === value ? "var(--paper-3)" : "none",
                border: "none", color: "var(--ink)",
                fontFamily: "var(--font-mono)", fontSize: "11px",
                cursor: "pointer", textAlign: "left",
              }}
              onMouseEnter={e => (e.currentTarget as HTMLElement).style.background = "var(--paper-3)"}
              onMouseLeave={e => (e.currentTarget as HTMLElement).style.background = opt === value ? "var(--paper-3)" : "none"}
            >
              {opt}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function ScriptStringVarRow({ label, value, onCommit }: { label: string; value: string; onCommit: (v: string) => void }) {
  const [focused, setFocused] = useState(false);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "8px", height: "28px", paddingLeft: 0 }}>
      <span style={{ width: "78px", fontSize: "12px", color: "var(--ink-3)", flexShrink: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{label}</span>
      <input
        defaultValue={value}
        key={value}
        onFocus={() => setFocused(true)}
        onBlur={e => { setFocused(false); onCommit(e.target.value); }}
        onKeyDown={e => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
        style={inputStyle(focused)}
      />
    </div>
  );
}
