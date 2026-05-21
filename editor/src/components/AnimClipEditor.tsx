import { useState, useEffect, useRef, useCallback } from "react";
import type { AnimClip } from "../App";

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
  margin?: number;
  spacing?: number;
}

interface Props {
  comp: AnimatedSpriteComp;
  onClose: () => void;
  onSave: (updated: Partial<AnimatedSpriteComp>) => void;
  onCommit?: (updated: Partial<AnimatedSpriteComp>) => void;
  saveLabel?: string;
}

function resolveTextureUrl(path: string): string {
  if (!path) return "";
  if (path.startsWith("http") || path.startsWith("data:")) return path;
  return `http://localhost:7878/assets/${path}`;
}

export default function AnimClipEditor({ comp, onClose, onSave, onCommit, saveLabel }: Props) {
  const [clips, setClips] = useState<AnimClip[]>(
    comp.clips.length > 0 ? comp.clips : [{ name: "idle", start_frame: 0, end_frame: Math.max(0, comp.cols * comp.rows - 1), fps: 10, looping: true }]
  );
  const [defaultClip, setDefaultClip] = useState(comp.default_clip);
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [selectStart, setSelectStart] = useState<number | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  const [previewFrame, setPreviewFrame] = useState(0);
  const [previewPlaying, setPreviewPlaying] = useState(true);
  const previewIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const previewCanvasRef = useRef<HTMLCanvasElement>(null);

  const [img, setImg] = useState<HTMLImageElement | null>(null);
  useEffect(() => {
    if (!comp.texture_path) return;
    const el = new Image();
    el.onload = () => setImg(el);
    el.onerror = () => setImg(null);
    el.src = resolveTextureUrl(comp.texture_path);
  }, [comp.texture_path]);

  const clip = clips[selectedIdx] ?? clips[0];
  const totalFrames = comp.cols * comp.rows;

  const frameUV = (idx: number) => {
    const col = idx % comp.cols;
    const row = Math.floor(idx / comp.cols);
    return { u: col / comp.cols, v: row / comp.rows, uw: 1 / comp.cols, uh: 1 / comp.rows };
  };

  const restartPreview = useCallback((c: AnimClip) => {
    if (previewIntervalRef.current) clearInterval(previewIntervalRef.current);
    setPreviewFrame(0);
    const frameCount = c.end_frame - c.start_frame + 1;
    if (!c || c.fps <= 0 || frameCount <= 0) return;
    let f = 0;
    previewIntervalRef.current = setInterval(() => {
      f = (f + 1) % frameCount;
      setPreviewFrame(f);
    }, 1000 / c.fps);
  }, []);

  useEffect(() => {
    if (previewPlaying && clip) restartPreview(clip);
    return () => { if (previewIntervalRef.current) clearInterval(previewIntervalRef.current); };
  }, [clip, previewPlaying, restartPreview]);

  useEffect(() => {
    const canvas = previewCanvasRef.current;
    if (!canvas || !img || !clip) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const absFrame = clip.start_frame + previewFrame;
    const { u, v, uw, uh } = frameUV(absFrame);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, u * img.naturalWidth, v * img.naturalHeight, uw * img.naturalWidth, uh * img.naturalHeight, 0, 0, canvas.width, canvas.height);
  }, [img, clip, previewFrame]);

  const CELL = Math.min(72, Math.max(32, Math.floor(540 / comp.cols)));
  const CELL_H = img ? Math.round(CELL * (img.naturalHeight / comp.rows) / (img.naturalWidth / comp.cols)) : CELL;

  const updateClip = (idx: number, fields: Partial<AnimClip>) =>
    setClips(prev => prev.map((c, i) => i === idx ? { ...c, ...fields } : c));

  const addClip = () => {
    const nc: AnimClip = { name: `clip_${clips.length + 1}`, start_frame: 0, end_frame: 0, fps: 10, looping: true };
    const next = [...clips, nc];
    setClips(next);
    setSelectedIdx(next.length - 1);
  };

  const removeClip = (i: number) => {
    if (clips.length <= 1) return;
    const next = clips.filter((_, ci) => ci !== i);
    setClips(next);
    setSelectedIdx(Math.min(selectedIdx, next.length - 1));
    if (defaultClip === clips[i].name) setDefaultClip(next[0].name);
  };

  const handleFrameMouseDown = (i: number, e: React.MouseEvent) => {
    e.preventDefault();
    setSelectStart(i);
    setIsDragging(true);
    updateClip(selectedIdx, { start_frame: i, end_frame: i });
  };

  const handleFrameMouseEnter = (i: number) => {
    if (!isDragging || selectStart === null) return;
    updateClip(selectedIdx, { start_frame: Math.min(selectStart, i), end_frame: Math.max(selectStart, i) });
  };

  const handleSave = () => {
    const patch = { clips, default_clip: defaultClip };
    if (onCommit) { onCommit(patch); } else { onSave(patch); onClose(); }
  };

  const thumbBg = (frameIdx: number) => {
    if (!comp.texture_path) return {};
    const { u, v, uw, uh } = frameUV(frameIdx);
    return {
      backgroundImage: `url(${resolveTextureUrl(comp.texture_path)})`,
      backgroundSize: `${comp.cols * 100}% ${comp.rows * 100}%`,
      backgroundPosition: `${uw > 0 ? u / (1 - uw) * 100 : 0}% ${uh > 0 ? v / (1 - uh) * 100 : 0}%`,
      backgroundRepeat: "no-repeat" as const,
    };
  };

  return (
    <div
      style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.72)", zIndex: 300, display: "flex", alignItems: "center", justifyContent: "center" }}
      onClick={onClose}
      onMouseUp={() => { setIsDragging(false); setSelectStart(null); }}
    >
      <div
        style={{ width: "960px", maxWidth: "calc(100vw - 32px)", height: "620px", maxHeight: "calc(100vh - 48px)", background: "var(--paper)", border: "1px solid var(--rule-2)", boxShadow: "0 32px 80px rgba(0,0,0,0.55)", display: "flex", flexDirection: "column", overflow: "hidden" }}
        onClick={e => e.stopPropagation()}
        onMouseUp={() => { setIsDragging(false); setSelectStart(null); }}
      >

        {/* ── Header ────────────────────────────────────────────────── */}
        <div style={{ height: "46px", display: "flex", alignItems: "center", gap: "10px", padding: "0 18px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
          <span style={{ color: "var(--amber)", fontSize: "11px" }}>▶</span>
          <span style={{ fontFamily: "var(--font-ui)", fontSize: "14px", color: "var(--ink)", fontWeight: 500 }}>Animation Clips</span>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>
            {comp.texture_path ? comp.texture_path.split("/").pop() : "(no texture)"}
          </span>
          <div style={{ width: "1px", height: "16px", background: "var(--rule-2)" }} />
          {/* Grid controls — moved to header */}
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>grid</span>
          <GridSpinner label="cols" value={comp.cols} onChange={v => onSave({ cols: v })} />
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>×</span>
          <GridSpinner label="rows" value={comp.rows} onChange={v => onSave({ rows: v })} />
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>= {totalFrames} frames</span>
          <div style={{ flex: 1 }} />
          <button onClick={onClose} style={{ background: "none", border: "none", color: "var(--ink-4)", cursor: "pointer", fontSize: "18px", lineHeight: 1, padding: "0 2px" }}>×</button>
        </div>

        {/* ── Body ──────────────────────────────────────────────────── */}
        <div style={{ flex: 1, display: "flex", minHeight: 0, overflow: "hidden" }}>

          {/* Left: Spritesheet grid */}
          <div style={{ flex: 1, overflow: "auto", padding: "14px 16px", borderRight: "1px solid var(--rule)" }}>
            <div style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", marginBottom: "10px" }}>
              click to set start · drag to select range
            </div>
            {comp.texture_path && img ? (
              <div style={{ display: "flex", gap: 0 }}>
                {/* Row number labels */}
                <div style={{ display: "flex", flexDirection: "column", marginRight: "6px", userSelect: "none" }}>
                  {Array.from({ length: comp.rows }, (_, r) => (
                    <div key={r} style={{ height: `${CELL_H}px`, display: "flex", alignItems: "center", justifyContent: "flex-end", paddingRight: "4px" }}>
                      <span style={{ fontFamily: "var(--font-mono)", fontSize: "9px", color: "var(--ink-4)" }}>{r}</span>
                    </div>
                  ))}
                </div>
                {/* Grid */}
                <div
                  style={{ display: "grid", gridTemplateColumns: `repeat(${comp.cols}, ${CELL}px)`, userSelect: "none", cursor: "crosshair" }}
                >
                  {Array.from({ length: totalFrames }, (_, i) => {
                    const inClip = clip && i >= clip.start_frame && i <= clip.end_frame;
                    const isStart = clip && i === clip.start_frame;
                    const isEnd = clip && i === clip.end_frame;
                    const { u, v, uw, uh } = frameUV(i);
                    return (
                      <div
                        key={i}
                        onMouseDown={e => handleFrameMouseDown(i, e)}
                        onMouseEnter={() => handleFrameMouseEnter(i)}
                        style={{
                          width: `${CELL}px`, height: `${CELL_H}px`, position: "relative", boxSizing: "border-box",
                          backgroundImage: `url(${resolveTextureUrl(comp.texture_path)})`,
                          backgroundSize: `${comp.cols * 100}% ${comp.rows * 100}%`,
                          backgroundPosition: `${uw > 0 ? u / (1 - uw) * 100 : 0}% ${uh > 0 ? v / (1 - uh) * 100 : 0}%`,
                          backgroundRepeat: "no-repeat",
                          outline: inClip ? "2px solid var(--amber)" : "1px solid rgba(255,255,255,0.06)",
                          outlineOffset: "-1px",
                          opacity: inClip ? 1 : 0.3,
                          transition: "opacity 0.08s",
                        }}
                      >
                        <span style={{ position: "absolute", bottom: "2px", right: "3px", fontFamily: "var(--font-mono)", fontSize: "8px", color: inClip ? "var(--amber)" : "rgba(255,255,255,0.4)", textShadow: "0 1px 2px #000", pointerEvents: "none" }}>{i}</span>
                        {(isStart || isEnd) && (
                          <span style={{ position: "absolute", top: "2px", left: "3px", fontFamily: "var(--font-mono)", fontSize: "8px", color: "var(--amber)", textShadow: "0 1px 2px #000", pointerEvents: "none" }}>
                            {isStart && isEnd ? "S/E" : isStart ? "S" : "E"}
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            ) : (
              <div style={{ width: "100%", height: "200px", display: "flex", alignItems: "center", justifyContent: "center", border: "1px solid var(--rule)", color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px" }}>
                {comp.texture_path ? "loading texture…" : "no texture set"}
              </div>
            )}
          </div>

          {/* Right: Clips + Editor + Preview */}
          <div style={{ width: "272px", flexShrink: 0, display: "flex", flexDirection: "column", minHeight: 0, overflow: "hidden" }}>

            {/* Clip list — scrollable */}
            <div style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0, borderBottom: "1px solid var(--rule)" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px 6px", flexShrink: 0 }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", letterSpacing: "0.05em" }}>CLIPS</span>
                <button onClick={addClip} style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "2px 8px", cursor: "pointer" }}>+ add</button>
              </div>
              <div style={{ overflowY: "auto", flex: 1, padding: "0 8px 8px" }}>
                {clips.map((c, i) => (
                  <ClipRow
                    key={i}
                    clip={c}
                    selected={i === selectedIdx}
                    isDefault={c.name === defaultClip}
                    thumbBg={comp.texture_path && img ? thumbBg(c.start_frame) : undefined}
                    onClick={() => setSelectedIdx(i)}
                    onRemove={clips.length > 1 ? () => removeClip(i) : undefined}
                  />
                ))}
              </div>
            </div>

            {/* Clip editor */}
            {clip && (
              <div style={{ padding: "12px 14px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", letterSpacing: "0.05em", display: "block", marginBottom: "10px" }}>EDIT CLIP</span>

                <EditorRow label="name">
                  <input
                    value={clip.name}
                    onChange={e => updateClip(selectedIdx, { name: e.target.value })}
                    style={inputSt}
                  />
                </EditorRow>

                <EditorRow label="frames">
                  <div style={{ display: "flex", alignItems: "center", gap: "4px", flex: 1 }}>
                    <input type="number" min={0} max={totalFrames - 1} value={clip.start_frame}
                      onChange={e => updateClip(selectedIdx, { start_frame: Math.min(clip.end_frame, Math.max(0, +e.target.value || 0)) })}
                      style={{ ...inputSt, width: "38px", textAlign: "center", padding: "3px 2px" }} />
                    <span style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px" }}>→</span>
                    <input type="number" min={0} max={totalFrames - 1} value={clip.end_frame}
                      onChange={e => updateClip(selectedIdx, { end_frame: Math.max(clip.start_frame, Math.max(0, +e.target.value || 0)) })}
                      style={{ ...inputSt, width: "38px", textAlign: "center", padding: "3px 2px" }} />
                    <span style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px", marginLeft: "2px", flexShrink: 0 }}>
                      {clip.end_frame - clip.start_frame + 1}f
                    </span>
                  </div>
                </EditorRow>

                <EditorRow label="fps">
                  <input type="range" min={1} max={60} step={1} value={clip.fps}
                    onChange={e => updateClip(selectedIdx, { fps: +e.target.value })}
                    style={{ flex: 1, accentColor: "var(--amber)", minWidth: 0 }} />
                  <input type="number" min={1} max={60} value={clip.fps}
                    onChange={e => updateClip(selectedIdx, { fps: Math.max(1, Math.min(60, +e.target.value || 1)) })}
                    style={{ ...inputSt, width: "34px", textAlign: "center", padding: "3px 2px", marginLeft: "4px" }} />
                </EditorRow>

                <EditorRow label="loop">
                  <input type="checkbox" checked={clip.looping}
                    onChange={e => updateClip(selectedIdx, { looping: e.target.checked })}
                    style={{ accentColor: "var(--amber)", width: "14px", height: "14px" }} />
                </EditorRow>

                <button
                  onClick={() => setDefaultClip(clip.name)}
                  style={{
                    marginTop: "10px", width: "100%",
                    background: clip.name === defaultClip ? "rgba(240,192,80,0.12)" : "transparent",
                    border: `1px solid ${clip.name === defaultClip ? "var(--amber)" : "var(--rule-2)"}`,
                    color: clip.name === defaultClip ? "var(--amber)" : "var(--ink-4)",
                    fontFamily: "var(--font-mono)", fontSize: "10px", padding: "5px 0", cursor: "pointer",
                    letterSpacing: "0.03em",
                  }}
                >{clip.name === defaultClip ? "★ default clip" : "set as default"}</button>
              </div>
            )}

            {/* Preview */}
            <div style={{ padding: "12px 14px", flexShrink: 0 }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", letterSpacing: "0.05em" }}>PREVIEW</span>
                <button onClick={() => setPreviewPlaying(p => !p)}
                  style={{ background: "none", border: "1px solid var(--rule)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "2px 8px", cursor: "pointer" }}>
                  {previewPlaying ? "⏸" : "▶"}
                </button>
              </div>
              <div style={{
                width: "100%", aspectRatio: "1", maxHeight: "100px",
                background: "repeating-conic-gradient(rgba(255,255,255,0.04) 0% 25%, transparent 0% 50%) 0 0 / 14px 14px",
                display: "flex", alignItems: "center", justifyContent: "center",
                border: "1px solid var(--rule)", overflow: "hidden",
              }}>
                {img && clip ? (
                  <canvas ref={previewCanvasRef} width={128} height={128}
                    style={{ imageRendering: "pixelated", maxWidth: "100%", maxHeight: "100%" }} />
                ) : (
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
                    {comp.texture_path ? "…" : "no texture"}
                  </span>
                )}
              </div>
              {clip && (
                <div style={{ marginTop: "5px", fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", textAlign: "center" }}>
                  {clip.start_frame + (previewFrame % Math.max(1, clip.end_frame - clip.start_frame + 1))} · {clip.fps}fps · {clip.looping ? "loop" : "once"}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ── Footer ────────────────────────────────────────────────── */}
        <div style={{ height: "50px", display: "flex", alignItems: "center", justifyContent: "flex-end", padding: "0 18px", gap: "10px", borderTop: "1px solid var(--rule)", flexShrink: 0 }}>
          <button onClick={onClose} style={{ background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)", fontFamily: "var(--font-ui)", fontSize: "12px", padding: "6px 18px", cursor: "pointer" }}>Cancel</button>
          <button onClick={handleSave} style={{ background: "var(--amber)", border: "none", color: "var(--paper)", fontFamily: "var(--font-ui)", fontSize: "12px", padding: "6px 18px", cursor: "pointer", fontWeight: 500 }}>{saveLabel ?? "Save Changes"}</button>
        </div>
      </div>
    </div>
  );
}

// ── Sub-components ────────────────────────────────────────────────

function ClipRow({ clip, selected, isDefault, thumbBg, onClick, onRemove }: {
  clip: AnimClip; selected: boolean; isDefault: boolean;
  thumbBg?: React.CSSProperties; onClick: () => void; onRemove?: () => void;
}) {
  const [hovered, setHovered] = useState(false);
  return (
    <div
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        display: "flex", alignItems: "center", gap: "8px",
        padding: "5px 6px", cursor: "pointer", borderRadius: "2px",
        background: selected ? "rgba(240,192,80,0.1)" : hovered ? "rgba(255,255,255,0.03)" : "transparent",
        border: `1px solid ${selected ? "var(--amber)" : "transparent"}`,
        marginBottom: "2px",
      }}
    >
      {/* Thumbnail */}
      <div style={{
        width: "22px", height: "22px", flexShrink: 0, border: "1px solid var(--rule)",
        imageRendering: "pixelated", overflow: "hidden",
        ...(thumbBg ?? { background: "var(--paper-2)" }),
      }} />
      {/* Name */}
      <span style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: "11px", color: selected ? "var(--amber)" : "var(--ink-3)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {isDefault ? "★ " : ""}{clip.name}
      </span>
      {/* Range */}
      <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", flexShrink: 0 }}>
        {clip.start_frame}–{clip.end_frame}
      </span>
      {/* Remove */}
      {onRemove && (
        <button
          onClick={e => { e.stopPropagation(); onRemove(); }}
          style={{ background: "none", border: "none", color: hovered ? "var(--ink-3)" : "transparent", cursor: "pointer", fontSize: "13px", padding: "0 1px", lineHeight: 1, flexShrink: 0, transition: "color 0.1s" }}
        >×</button>
      )}
    </div>
  );
}

function GridSpinner({ label: _label, value, onChange }: { label: string; value: number; onChange: (v: number) => void }) {
  return (
    <input
      type="number" min={1} value={value}
      onChange={e => onChange(Math.max(1, parseInt(e.target.value) || 1))}
      style={{ width: "40px", background: "var(--paper-2)", border: "1px solid var(--rule)", color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "2px 4px", textAlign: "center" }}
    />
  );
}

function EditorRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "7px", minHeight: "24px" }}>
      <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", width: "40px", flexShrink: 0 }}>{label}</span>
      {children}
    </div>
  );
}

const inputSt: React.CSSProperties = {
  flex: 1, background: "var(--paper)", border: "1px solid var(--rule)",
  color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "3px 6px",
};
