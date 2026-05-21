import { useState, useEffect, useRef } from "react";

export interface ProjectFile {
  path: string;
  kind: string;
  name: string;
}

interface Props {
  title: string;
  kinds: string[];
  files: ProjectFile[];
  onSelect: (path: string) => void;
  onClose: () => void;
}

const KIND_ICON: Record<string, string> = {
  image:      "▣",
  animclips:  "▶",
  script:     "⚡",
  scene:      "◈",
  audio:      "♪",
  other:      "·",
};

export default function FilePicker({ title, kinds, files, onSelect, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [focused, setFocused] = useState(0);
  const searchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const filtered = files.filter(f =>
    kinds.includes(f.kind) &&
    (query === "" || f.name.toLowerCase().includes(query.toLowerCase()) || f.path.toLowerCase().includes(query.toLowerCase()))
  );

  useEffect(() => { searchRef.current?.focus(); }, []);
  useEffect(() => { setFocused(0); }, [query]);

  useEffect(() => {
    const el = listRef.current?.children[focused] as HTMLElement | undefined;
    el?.scrollIntoView({ block: "nearest" });
  }, [focused]);

  const commit = (path: string) => { onSelect(path); onClose(); };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown")  { e.preventDefault(); setFocused(i => Math.min(i + 1, filtered.length - 1)); }
    if (e.key === "ArrowUp")    { e.preventDefault(); setFocused(i => Math.max(i - 1, 0)); }
    if (e.key === "Enter")      { if (filtered[focused]) commit(filtered[focused].path); }
    if (e.key === "Escape")     { onClose(); }
  };

  return (
    <div
      style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)", zIndex: 400, display: "flex", alignItems: "center", justifyContent: "center" }}
      onClick={onClose}
    >
      <div
        style={{ width: "460px", background: "var(--paper)", border: "1px solid var(--rule-2)", boxShadow: "0 24px 64px rgba(0,0,0,0.5)", display: "flex", flexDirection: "column", maxHeight: "60vh" }}
        onClick={e => e.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        {/* Header */}
        <div style={{ padding: "12px 14px 8px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
          <div style={{ fontFamily: "var(--font-ui)", fontSize: "12px", color: "var(--ink-3)", marginBottom: "8px" }}>{title}</div>
          <input
            ref={searchRef}
            value={query}
            onChange={e => setQuery(e.target.value)}
            placeholder="Search files…"
            style={{
              width: "100%", boxSizing: "border-box",
              background: "var(--bg-2, var(--paper-2))", border: "1px solid var(--rule)",
              color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "12px",
              padding: "6px 8px", outline: "none",
            }}
          />
        </div>

        {/* File list */}
        <div ref={listRef} style={{ flex: 1, overflow: "auto", padding: "4px 0" }}>
          {filtered.length === 0 ? (
            <div style={{ padding: "16px 14px", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px" }}>
              No {kinds.join(" / ")} files in project.
            </div>
          ) : filtered.map((f, i) => (
            <div
              key={f.path}
              onClick={() => commit(f.path)}
              onMouseEnter={() => setFocused(i)}
              style={{
                display: "flex", alignItems: "center", gap: "8px",
                padding: "6px 14px", cursor: "pointer",
                background: i === focused ? "var(--accent-glow, var(--paper-3))" : "transparent",
                borderLeft: `2px solid ${i === focused ? "var(--accent)" : "transparent"}`,
              }}
            >
              <span style={{ fontSize: "11px", color: "var(--ink-4)", width: "14px", textAlign: "center", flexShrink: 0 }}>
                {KIND_ICON[f.kind] ?? "·"}
              </span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {f.name}
              </span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: "180px" }}>
                {f.path}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
