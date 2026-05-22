import type { CSSProperties } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ForgeLogo from "./ForgeLogo";
import { KbdKey } from "./KbdKey";
import type { ActiveTool } from "../App";

export interface TopbarProps {
  sceneName: string;
  projectName: string;
  sceneDirty: boolean;
  playbackState: "stopped" | "playing" | "paused";
  engineReady: boolean;
  onPlayback: (action: "play" | "pause" | "stop") => void;
  onOpenCmdK: () => void;
  activeTool: ActiveTool;
  setActiveTool: (t: ActiveTool) => void;
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
  onOpenSettings: () => void;
}

export default function Topbar({
  sceneName, projectName, sceneDirty,
  playbackState, engineReady, onPlayback,
  onOpenCmdK,
  activeTool, setActiveTool,
  canUndo, canRedo, onUndo, onRedo,
  onOpenSettings,
}: TopbarProps) {
  const tools: { key: ActiveTool; icon: string; label: string }[] = [
    { key: "select",   icon: "↖", label: "Select" },
    { key: "move",     icon: "✥", label: "Move" },
    { key: "scale",    icon: "⤢", label: "Scale" },
    { key: "rotate",   icon: "↻", label: "Rotate" },
    { key: "collider", icon: "⬡", label: "Edit Collider" },
  ];

  const win = getCurrentWindow();

  const drag = () => win.startDragging();

  return (
    <header
      style={{
        height: "var(--header-h)",
        display: "grid",
        gridTemplateColumns: "380px 1fr 300px",
        borderBottom: "1px solid var(--rule-2)",
        background: "var(--paper)",
        alignItems: "center",
        flexShrink: 0,
        userSelect: "none",
      }}
    >
      {/* Left: Logo + breadcrumb — acts as drag handle */}
      <div
        onMouseDown={drag}
        style={{
          display: "flex", alignItems: "center",
          padding: "0 18px", gap: "12px",
          height: "100%", overflow: "hidden", minWidth: 0,
          cursor: "default",
        }}
      >
        <ForgeLogo size={22} />
        <span style={{
          fontFamily: "var(--font-ui)", fontWeight: 700, fontSize: "18px",
          letterSpacing: "0.14em", color: "var(--ink)",
          display: "flex", alignItems: "center", gap: "1px",
          flex: "none",
        }}>
          S<span style={{ color: "var(--amber)" }}>I</span>NDRI
        </span>
        <div style={{ width: "1px", height: "20px", background: "var(--rule-2)", flex: "none" }} />
        <div style={{
          fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-3)",
          display: "flex", alignItems: "baseline", gap: "6px",
          overflow: "hidden", minWidth: 0,
          whiteSpace: "nowrap",
        }}>
          <span style={{ flex: "none" }}>scenes</span>
          <span style={{ color: "var(--ink-4)", flex: "none" }}>/</span>
          <span style={{
            color: "var(--ink)",
            overflow: "hidden", textOverflow: "ellipsis",
            minWidth: 0, flex: "1 1 auto",
          }}>
            {sceneName || projectName || "untitled"}{sceneDirty ? " *" : ""}
          </span>
        </div>
      </div>

      {/* Center: Cmd-K + tools + run controls */}
      <div style={{
        display: "flex", alignItems: "center",
        padding: "0 18px", gap: "12px", height: "100%",
      }}>
        <button
          onClick={onOpenCmdK}
          style={{
            display: "flex", alignItems: "center", gap: "10px",
            height: "32px", padding: "0 14px",
            background: "var(--paper-2)",
            border: "1px solid var(--rule)",
            color: "var(--ink-3)",
            fontSize: "13px", fontFamily: "var(--font-ui)",
            flex: 1, maxWidth: "380px",
            cursor: "text",
          }}
        >
          <SparkleIcon size={14} />
          <span style={{ flex: 1, textAlign: "left" }}>Ask Sindri to build something…</span>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>
            <KbdKey>Ctrl</KbdKey><KbdKey>K</KbdKey>
          </span>
        </button>

        <div style={{ display: "flex", gap: "2px" }}>
          {tools.map(t => (
            <button key={t.key} title={t.label} onClick={() => setActiveTool(t.key)} style={{
              width: "28px", height: "28px",
              display: "inline-flex", alignItems: "center", justifyContent: "center",
              background: activeTool === t.key ? "var(--paper-3)" : "transparent",
              border: activeTool === t.key ? "1px solid var(--rule-2)" : "1px solid transparent",
              color: activeTool === t.key ? "var(--ink)" : "var(--ink-3)",
              fontSize: "13px", cursor: "pointer",
              fontFamily: "var(--font-mono)",
            }}>{t.icon}</button>
          ))}
        </div>

        <div style={{ width: "1px", height: "18px", background: "var(--rule-2)" }} />

        <button title="Undo (Ctrl+Z)" onClick={canUndo ? onUndo : undefined} style={iconBtnStyle(canUndo)}>↶</button>
        <button title="Redo (Ctrl+Y)" onClick={canRedo ? onRedo : undefined} style={iconBtnStyle(canRedo)}>↷</button>

        <div style={{ width: "1px", height: "18px", background: "var(--rule-2)" }} />

        <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
          <RunBtn
            title={playbackState === "paused" ? "Resume" : "Play"}
            disabled={!engineReady || playbackState === "playing"}
            variant="play"
            onClick={() => onPlayback("play")}
          />
          <RunBtn
            title="Pause"
            disabled={!engineReady || playbackState !== "playing"}
            variant="pause"
            onClick={() => onPlayback("pause")}
          />
          <RunBtn
            title="Stop"
            disabled={!engineReady || playbackState === "stopped"}
            variant="stop"
            onClick={() => onPlayback("stop")}
          />
        </div>
      </div>

      {/* Right: compose + settings + window controls */}
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "flex-end",
        height: "100%", gap: "0",
      }}>
        <button
          onClick={onOpenCmdK}
          style={{
            display: "inline-flex", alignItems: "center", gap: "6px",
            fontFamily: "var(--font-ui)", fontSize: "14px", color: "var(--amber)",
            background: "none", border: "none", cursor: "pointer",
            padding: "0 10px",
          }}
        >
          <SparkleIcon size={13} /> Compose
        </button>

        <div style={{ width: "1px", height: "18px", background: "var(--rule-2)", margin: "0 4px" }} />

        <button
          onClick={onOpenSettings}
          title="Settings"
          style={{
            width: "34px", height: "100%",
            background: "transparent",
            border: "none",
            borderLeft: "1px solid transparent",
            color: "var(--ink-3)",
            cursor: "pointer",
            fontFamily: "var(--font-mono)",
            fontSize: "14px",
          }}
        >⊞</button>

        <div style={{ width: "1px", height: "18px", background: "var(--rule-2)" }} />

        {/* Window controls */}
        <button
          title="Minimize"
          onClick={() => win.minimize()}
          style={winBtnStyle(false)}
          onMouseEnter={e => (e.currentTarget as HTMLElement).style.background = "var(--paper-3)"}
          onMouseLeave={e => (e.currentTarget as HTMLElement).style.background = "transparent"}
        >─</button>
        <button
          title="Maximize / Restore"
          onClick={() => win.toggleMaximize()}
          style={winBtnStyle(false)}
          onMouseEnter={e => (e.currentTarget as HTMLElement).style.background = "var(--paper-3)"}
          onMouseLeave={e => (e.currentTarget as HTMLElement).style.background = "transparent"}
        >□</button>
        <button
          title="Close"
          onClick={() => win.close()}
          style={winBtnStyle(true)}
          onMouseEnter={e => { (e.currentTarget as HTMLElement).style.background = "#c0392b"; (e.currentTarget as HTMLElement).style.color = "white"; }}
          onMouseLeave={e => { (e.currentTarget as HTMLElement).style.background = "transparent"; (e.currentTarget as HTMLElement).style.color = "var(--ink-3)"; }}
        >✕</button>
      </div>
    </header>
  );
}

function winBtnStyle(isClose: boolean): CSSProperties {
  return {
    width: "46px", height: "100%",
    display: "inline-flex", alignItems: "center", justifyContent: "center",
    background: "transparent",
    border: "none",
    color: "var(--ink-3)",
    fontFamily: "var(--font-mono)",
    fontSize: isClose ? "11px" : "14px",
    cursor: "pointer",
  };
}

function iconBtnStyle(enabled: boolean): CSSProperties {
  return {
    width: "26px", height: "26px",
    display: "inline-flex", alignItems: "center", justifyContent: "center",
    background: "transparent",
    border: "none",
    color: enabled ? "var(--ink-2)" : "var(--ink-4)",
    fontFamily: "var(--font-mono)",
    fontSize: "14px",
    cursor: enabled ? "pointer" : "default",
  };
}

function RunBtn({ title, disabled, variant, onClick }: {
  title: string;
  disabled: boolean;
  variant: "play" | "pause" | "stop";
  onClick: () => void;
}) {
  const isPlay = variant === "play";
  return (
    <button
      onClick={disabled ? undefined : onClick}
      title={title}
      style={{
        width: "34px", height: "28px",
        display: "inline-flex", alignItems: "center", justifyContent: "center",
        cursor: disabled ? "default" : "pointer",
        background: isPlay && !disabled ? "var(--ink)" : "transparent",
        color: disabled
          ? "var(--ink-4)"
          : isPlay ? "var(--paper)" : "var(--ink-3)",
        border: isPlay && !disabled
          ? "1px solid var(--ink)"
          : "1px solid var(--rule-2)",
      }}
    >
      {variant === "play" && (
        <span style={{
          width: 0, height: 0,
          borderTop: "5px solid transparent",
          borderBottom: "5px solid transparent",
          borderLeft: `8px solid currentColor`,
          display: "inline-block",
        }} />
      )}
      {variant === "pause" && (
        <span style={{ display: "flex", gap: "2px" }}>
          <span style={{ width: "3px", height: "10px", background: "currentColor", display: "inline-block" }} />
          <span style={{ width: "3px", height: "10px", background: "currentColor", display: "inline-block" }} />
        </span>
      )}
      {variant === "stop" && (
        <span style={{ width: "9px", height: "9px", background: "currentColor", display: "inline-block" }} />
      )}
    </button>
  );
}

function SparkleIcon({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" style={{ flexShrink: 0 }}>
      <path d="M8 1v14M1 8h14M3.5 3.5l9 9M12.5 3.5l-9 9" stroke="var(--amber)" strokeWidth="1.4" strokeLinecap="square" />
    </svg>
  );
}
