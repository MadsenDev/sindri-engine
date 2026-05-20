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
}

interface Props {
  comp: AnimatedSpriteComp;
  onClose: () => void;
  onSave: (updated: Partial<AnimatedSpriteComp>) => void;
}

function resolveTextureUrl(path: string): string {
  if (!path) return "";
  if (path.startsWith("http") || path.startsWith("data:")) return path;
  if (path.startsWith("/")) return path;
  return `http://localhost:7878/assets/${path}`;
}

export default function AnimClipEditor({ comp, onClose, onSave }: Props) {
  const [clips, setClips] = useState<AnimClip[]>(
    comp.clips.length > 0 ? comp.clips : [{ name: "idle", start_frame: 0, end_frame: Math.max(0, comp.cols * comp.rows - 1), fps: 10, looping: true }]
  );
  const [defaultClip, setDefaultClip] = useState(comp.default_clip);
  const [selectedClipIdx, setSelectedClipIdx] = useState(0);
  const [selectStart, setSelectStart] = useState<number | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  // Preview animation state
  const [previewFrame, setPreviewFrame] = useState(0);
  const [previewPlaying, setPreviewPlaying] = useState(true);
  const previewIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Image loading
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  useEffect(() => {
    if (!comp.texture_path) return;
    const el = new Image();
    el.onload = () => setImg(el);
    el.onerror = () => setImg(null);
    el.src = resolveTextureUrl(comp.texture_path);
  }, [comp.texture_path]);

  const clip = clips[selectedClipIdx] ?? clips[0];

  // Preview playback
  const restartPreview = useCallback((c: AnimClip) => {
    if (previewIntervalRef.current) clearInterval(previewIntervalRef.current);
    setPreviewFrame(0);
    if (!c || c.fps <= 0) return;
    const ms = 1000 / c.fps;
    const frameCount = c.end_frame - c.start_frame + 1;
    if (frameCount <= 0) return;
    let f = 0;
    previewIntervalRef.current = setInterval(() => {
      f = (f + 1) % frameCount;
      setPreviewFrame(f);
    }, ms);
  }, []);

  useEffect(() => {
    if (previewPlaying && clip) restartPreview(clip);
    return () => { if (previewIntervalRef.current) clearInterval(previewIntervalRef.current); };
  }, [clip, previewPlaying, restartPreview]);

  const totalFrames = comp.cols * comp.rows;

  // Frame UV helpers
  const frameUV = (frameIdx: number) => {
    const col = frameIdx % comp.cols;
    const row = Math.floor(frameIdx / comp.cols);
    return { col, row, u: col / comp.cols, v: row / comp.rows, uw: 1 / comp.cols, uh: 1 / comp.rows };
  };

  // Selection handling
  const handleFrameMouseDown = (frameIdx: number, e: React.MouseEvent) => {
    e.preventDefault();
    setSelectStart(frameIdx);
    setIsDragging(true);
    setClips(prev => prev.map((c, i) => i === selectedClipIdx
      ? { ...c, start_frame: frameIdx, end_frame: frameIdx }
      : c
    ));
  };

  const handleFrameMouseEnter = (frameIdx: number) => {
    if (!isDragging || selectStart === null) return;
    const lo = Math.min(selectStart, frameIdx);
    const hi = Math.max(selectStart, frameIdx);
    setClips(prev => prev.map((c, i) => i === selectedClipIdx
      ? { ...c, start_frame: lo, end_frame: hi }
      : c
    ));
  };

  const handleMouseUp = () => {
    setIsDragging(false);
    setSelectStart(null);
  };

  const updateClip = (idx: number, fields: Partial<AnimClip>) => {
    setClips(prev => prev.map((c, i) => i === idx ? { ...c, ...fields } : c));
  };

  const addClip = () => {
    const newClip: AnimClip = { name: `clip${clips.length + 1}`, start_frame: 0, end_frame: 0, fps: 10, looping: true };
    const next = [...clips, newClip];
    setClips(next);
    setSelectedClipIdx(next.length - 1);
  };

  const removeClip = (i: number) => {
    if (clips.length <= 1) return;
    const next = clips.filter((_, ci) => ci !== i);
    setClips(next);
    setSelectedClipIdx(Math.min(selectedClipIdx, next.length - 1));
    if (defaultClip === clips[i].name) setDefaultClip(next[0].name);
  };

  const handleSave = () => {
    onSave({ clips, default_clip: defaultClip });
    onClose();
  };

  // Preview canvas
  const previewCanvasRef = useRef<HTMLCanvasElement>(null);
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

  // Spritesheet grid cell size for display
  const GRID_MAX_W = 520;
  const GRID_MAX_H = 340;
  const cellW = Math.min(80, Math.floor(GRID_MAX_W / comp.cols));
  const cellH = img ? Math.round(cellW * (img.naturalHeight / comp.rows) / (img.naturalWidth / comp.cols)) : cellW;
  const gridW = cellW * comp.cols;
  const gridH = Math.min(cellH * comp.rows, GRID_MAX_H);

  const isInClip = (frameIdx: number) => clip && frameIdx >= clip.start_frame && frameIdx <= clip.end_frame;

  return (
    <div
      style={{
        position: "fixed", inset: 0,
        background: "rgba(0,0,0,0.7)",
        zIndex: 300,
        display: "flex", alignItems: "center", justifyContent: "center",
      }}
      onClick={onClose}
      onMouseUp={handleMouseUp}
    >
      <div
        style={{
          width: "900px", maxWidth: "calc(100vw - 40px)",
          maxHeight: "calc(100vh - 60px)",
          background: "var(--paper)",
          border: "1px solid var(--rule-2)",
          boxShadow: "0 32px 80px rgba(0,0,0,0.5)",
          display: "flex", flexDirection: "column",
          overflow: "hidden",
        }}
        onClick={e => e.stopPropagation()}
        onMouseUp={handleMouseUp}
      >
        {/* Header */}
        <div style={{
          height: "44px", display: "flex", alignItems: "center",
          padding: "0 20px", gap: "12px",
          borderBottom: "1px solid var(--rule)",
          flexShrink: 0,
        }}>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--amber)" }}>▶</span>
          <span style={{ fontFamily: "var(--font-ui)", fontSize: "14px", color: "var(--ink)" }}>
            Animation Clips
          </span>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>
            — {comp.texture_path ? comp.texture_path.split("/").pop() : "(no texture)"} · {comp.cols}×{comp.rows} grid · {totalFrames} frames
          </span>
          <div style={{ flex: 1 }} />
          <button onClick={onClose} style={{ background: "none", border: "none", color: "var(--ink-4)", cursor: "pointer", fontSize: "16px", lineHeight: 1, padding: "0 4px" }}>×</button>
        </div>

        {/* Body */}
        <div style={{ display: "flex", flex: 1, overflow: "hidden", minHeight: 0 }}>

          {/* Left: Spritesheet grid */}
          <div style={{
            flex: 1, overflow: "auto",
            padding: "16px",
            borderRight: "1px solid var(--rule)",
            display: "flex", flexDirection: "column", gap: "10px",
          }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
              click to set start frame · drag to select range
            </span>

            {comp.texture_path && img ? (
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: `repeat(${comp.cols}, ${cellW}px)`,
                  width: `${gridW}px`,
                  maxHeight: `${gridH}px`,
                  overflow: "hidden",
                  userSelect: "none",
                  cursor: "crosshair",
                }}
              >
                {Array.from({ length: totalFrames }, (_, i) => {
                  const { u, v, uw, uh } = frameUV(i);
                  const inRange = isInClip(i);
                  const isStart = clip && i === clip.start_frame;
                  const isEnd = clip && i === clip.end_frame;
                  return (
                    <div
                      key={i}
                      onMouseDown={e => handleFrameMouseDown(i, e)}
                      onMouseEnter={() => handleFrameMouseEnter(i)}
                      style={{
                        position: "relative",
                        width: `${cellW}px`,
                        height: `${cellH}px`,
                        backgroundImage: `url(${resolveTextureUrl(comp.texture_path)})`,
                        backgroundSize: `${comp.cols * 100}% ${comp.rows * 100}%`,
                        backgroundPosition: `${u / (1 - uw) * 100}% ${v / (1 - uh) * 100}%`,
                        outline: inRange ? `2px solid var(--amber)` : "1px solid var(--rule)",
                        outlineOffset: "-1px",
                        background: inRange
                          ? `url(${resolveTextureUrl(comp.texture_path)})`
                          : undefined,
                        filter: inRange ? "none" : "brightness(0.45)",
                        boxSizing: "border-box",
                        overflow: "hidden",
                      }}
                    >
                      {/* Frame number */}
                      <span style={{
                        position: "absolute", bottom: "2px", right: "3px",
                        fontFamily: "var(--font-mono)", fontSize: "9px",
                        color: inRange ? "var(--amber)" : "rgba(255,255,255,0.5)",
                        textShadow: "0 1px 2px rgba(0,0,0,0.9)",
                        pointerEvents: "none",
                      }}>{i}</span>
                      {/* Start/end markers */}
                      {(isStart || isEnd) && (
                        <span style={{
                          position: "absolute", top: "2px", left: "3px",
                          fontFamily: "var(--font-mono)", fontSize: "8px",
                          color: "var(--amber)",
                          textShadow: "0 1px 2px rgba(0,0,0,0.9)",
                          pointerEvents: "none",
                        }}>{isStart && isEnd ? "S/E" : isStart ? "S" : "E"}</span>
                      )}
                    </div>
                  );
                })}
              </div>
            ) : (
              <div style={{
                width: `${gridW}px`, height: "160px",
                display: "flex", alignItems: "center", justifyContent: "center",
                border: "1px solid var(--rule)", color: "var(--ink-4)",
                fontFamily: "var(--font-mono)", fontSize: "11px",
              }}>
                {comp.texture_path ? "loading texture…" : "no texture set"}
              </div>
            )}

            {/* Grid size controls */}
            <div style={{ display: "flex", gap: "16px", alignItems: "center" }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>grid</span>
              <label style={{ display: "flex", alignItems: "center", gap: "4px", fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-3)" }}>
                cols
                <input
                  type="number" min={1} value={comp.cols}
                  onChange={e => onSave({ cols: Math.max(1, parseInt(e.target.value) || 1) })}
                  style={{ width: "40px", background: "var(--paper)", border: "1px solid var(--rule)", color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "2px 4px", textAlign: "center" }}
                />
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: "4px", fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-3)" }}>
                rows
                <input
                  type="number" min={1} value={comp.rows}
                  onChange={e => onSave({ rows: Math.max(1, parseInt(e.target.value) || 1) })}
                  style={{ width: "40px", background: "var(--paper)", border: "1px solid var(--rule)", color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "2px 4px", textAlign: "center" }}
                />
              </label>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>= {totalFrames} frames</span>
            </div>
          </div>

          {/* Right: Clips + Editor + Preview */}
          <div style={{ width: "280px", display: "flex", flexDirection: "column", flexShrink: 0, overflow: "hidden" }}>

            {/* Clips list */}
            <div style={{ padding: "12px 14px 8px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>CLIPS</span>
                <button onClick={addClip} style={{
                  background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)",
                  fontFamily: "var(--font-mono)", fontSize: "10px", padding: "2px 8px", cursor: "pointer",
                }}>+ add</button>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "2px" }}>
                {clips.map((c, i) => (
                  <div
                    key={i}
                    onClick={() => setSelectedClipIdx(i)}
                    style={{
                      display: "flex", alignItems: "center", gap: "6px",
                      padding: "5px 8px", cursor: "pointer",
                      background: i === selectedClipIdx ? "rgba(240,192,80,0.1)" : "transparent",
                      border: `1px solid ${i === selectedClipIdx ? "var(--amber)" : "transparent"}`,
                    }}
                  >
                    <span style={{
                      fontFamily: "var(--font-mono)", fontSize: "11px",
                      color: i === selectedClipIdx ? "var(--amber)" : "var(--ink-3)",
                      flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                    }}>
                      {c.name === defaultClip ? "★ " : ""}{c.name}
                    </span>
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", flexShrink: 0 }}>
                      {c.start_frame}–{c.end_frame}
                    </span>
                    {clips.length > 1 && (
                      <button onClick={e => { e.stopPropagation(); removeClip(i); }} style={{
                        background: "none", border: "none", color: "var(--ink-4)", cursor: "pointer",
                        fontSize: "12px", padding: "0 2px", lineHeight: 1, flexShrink: 0,
                      }}>×</button>
                    )}
                  </div>
                ))}
              </div>
            </div>

            {/* Selected clip editor */}
            {clip && (
              <div style={{ padding: "12px 14px", borderBottom: "1px solid var(--rule)", flexShrink: 0 }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", display: "block", marginBottom: "8px" }}>EDIT CLIP</span>

                <ClipField label="name">
                  <input
                    value={clip.name}
                    onChange={e => updateClip(selectedClipIdx, { name: e.target.value })}
                    style={fieldInputStyle}
                  />
                </ClipField>

                <ClipField label="frames">
                  <div style={{ display: "flex", alignItems: "center", gap: "4px", flex: 1 }}>
                    <input
                      type="number" min={0} max={totalFrames - 1}
                      value={clip.start_frame}
                      onChange={e => updateClip(selectedClipIdx, { start_frame: Math.min(clip.end_frame, Math.max(0, parseInt(e.target.value) || 0)) })}
                      style={{ ...fieldInputStyle, width: "40px", textAlign: "center" }}
                    />
                    <span style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px" }}>→</span>
                    <input
                      type="number" min={0} max={totalFrames - 1}
                      value={clip.end_frame}
                      onChange={e => updateClip(selectedClipIdx, { end_frame: Math.max(clip.start_frame, Math.max(0, parseInt(e.target.value) || 0)) })}
                      style={{ ...fieldInputStyle, width: "40px", textAlign: "center" }}
                    />
                    <span style={{ color: "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "10px", marginLeft: "2px" }}>
                      ({clip.end_frame - clip.start_frame + 1}f)
                    </span>
                  </div>
                </ClipField>

                <ClipField label="fps">
                  <div style={{ display: "flex", alignItems: "center", gap: "6px", flex: 1 }}>
                    <input
                      type="range" min={1} max={60} step={1}
                      value={clip.fps}
                      onChange={e => updateClip(selectedClipIdx, { fps: parseFloat(e.target.value) })}
                      style={{ flex: 1, accentColor: "var(--amber)" }}
                    />
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink)", width: "24px", textAlign: "right" }}>{clip.fps}</span>
                  </div>
                </ClipField>

                <ClipField label="loop">
                  <input
                    type="checkbox" checked={clip.looping}
                    onChange={e => updateClip(selectedClipIdx, { looping: e.target.checked })}
                    style={{ accentColor: "var(--amber)" }}
                  />
                </ClipField>

                <button
                  onClick={() => setDefaultClip(clip.name)}
                  style={{
                    marginTop: "8px", width: "100%",
                    background: clip.name === defaultClip ? "var(--amber)" : "transparent",
                    border: `1px solid ${clip.name === defaultClip ? "var(--amber)" : "var(--rule-2)"}`,
                    color: clip.name === defaultClip ? "var(--paper)" : "var(--ink-4)",
                    fontFamily: "var(--font-mono)", fontSize: "10px",
                    padding: "4px 0", cursor: "pointer",
                  }}
                >
                  {clip.name === defaultClip ? "★ default clip" : "set as default"}
                </button>
              </div>
            )}

            {/* Preview */}
            <div style={{ padding: "12px 14px", flex: 1 }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>PREVIEW</span>
                <button
                  onClick={() => setPreviewPlaying(p => !p)}
                  style={{ background: "none", border: "1px solid var(--rule)", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "10px", padding: "2px 8px", cursor: "pointer" }}
                >
                  {previewPlaying ? "⏸" : "▶"}
                </button>
              </div>
              <div style={{
                width: "100%", aspectRatio: "1",
                background: "repeating-conic-gradient(rgba(255,255,255,0.04) 0% 25%, transparent 0% 50%) 0 0 / 16px 16px",
                display: "flex", alignItems: "center", justifyContent: "center",
                border: "1px solid var(--rule)",
                overflow: "hidden",
              }}>
                {img && clip ? (
                  <canvas
                    ref={previewCanvasRef}
                    width={128}
                    height={128}
                    style={{ imageRendering: "pixelated", maxWidth: "100%", maxHeight: "100%" }}
                  />
                ) : (
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
                    {comp.texture_path ? "…" : "no texture"}
                  </span>
                )}
              </div>
              {clip && (
                <div style={{ marginTop: "6px", fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", textAlign: "center" }}>
                  frame {clip.start_frame + (previewFrame % Math.max(1, clip.end_frame - clip.start_frame + 1))} · {clip.fps} fps · {clip.looping ? "loop" : "once"}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div style={{
          height: "50px", display: "flex", alignItems: "center", justifyContent: "flex-end",
          padding: "0 20px", gap: "10px",
          borderTop: "1px solid var(--rule)", flexShrink: 0,
        }}>
          <button onClick={onClose} style={{
            background: "none", border: "1px solid var(--rule-2)", color: "var(--ink-3)",
            fontFamily: "var(--font-ui)", fontSize: "12px", padding: "6px 18px", cursor: "pointer",
          }}>Cancel</button>
          <button onClick={handleSave} style={{
            background: "var(--amber)", border: "1px solid var(--amber)", color: "var(--paper)",
            fontFamily: "var(--font-ui)", fontSize: "12px", padding: "6px 18px", cursor: "pointer",
          }}>Save Changes</button>
        </div>
      </div>
    </div>
  );
}

const fieldInputStyle: React.CSSProperties = {
  flex: 1, background: "var(--paper)", border: "1px solid var(--rule)",
  color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", padding: "3px 6px",
};

function ClipField({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "6px", minHeight: "26px" }}>
      <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", width: "44px", flexShrink: 0 }}>{label}</span>
      {children}
    </div>
  );
}
