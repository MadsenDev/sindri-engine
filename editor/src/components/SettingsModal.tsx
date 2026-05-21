import React, { useState, useEffect, useRef, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";

// ─── Types ───────────────────────────────────────────────────────────────────

interface ProjectSettings {
  name: string;
  resolution_width: number;
  resolution_height: number;
  pixel_art_mode: boolean;
}

interface EditorPrefs {
  auto_save_interval_secs: number;
}

interface WorldSettings {
  gravity_x: number;
  gravity_y: number;
}

export type AiModelRole = "assistant" | "code" | "fast" | "vision";
export type AiModelConfig = Record<AiModelRole, string | null>;
export type AiProvider = "ollama" | "openai" | "anthropic";

export interface AiProviderStatus {
  provider: AiProvider;
  configured: boolean;
  defaultModel?: string;
}

export type SettingsTab = "project" | "editor" | "ai";

// ─── Props ───────────────────────────────────────────────────────────────────

interface Props {
  open: boolean;
  onClose: () => void;
  initialTab?: SettingsTab;

  // Project / Editor props
  projectPath: string | null;
  engineReady: boolean;

  // AI props
  provider: AiProvider;
  setProvider: (p: AiProvider) => void;
  modelConfig: AiModelConfig;
  setModelForRole: (role: AiModelRole, model: string | null) => void;
  ollamaModels: string[];
  providerStatuses: AiProviderStatus[];
  onRefreshProviders: () => Promise<void>;

  // AI role metadata
  aiModelRoles: { role: AiModelRole; title: string; detail: string }[];
  preferredOllamaModel: (models: string[], role?: AiModelRole) => string | null;
}

// ─── Component ───────────────────────────────────────────────────────────────

const TABS: { key: SettingsTab; label: string; icon: string }[] = [
  { key: "project", label: "Project", icon: "◈" },
  { key: "editor",  label: "Editor",  icon: "⊞" },
  { key: "ai",      label: "AI",      icon: "✦" },
];

export default function SettingsModal({
  open, onClose, initialTab = "project",
  projectPath, engineReady,
  provider, setProvider, modelConfig, setModelForRole,
  ollamaModels, providerStatuses, onRefreshProviders,
  aiModelRoles, preferredOllamaModel,
}: Props) {
  const [tab, setTab] = useState<SettingsTab>(initialTab);
  const [proj, setProj] = useState<ProjectSettings>({ name: "My Game", resolution_width: 1280, resolution_height: 720, pixel_art_mode: true });
  const [world, setWorld] = useState<WorldSettings>({ gravity_x: 0, gravity_y: 980 });
  const [prefs, setPrefs] = useState<EditorPrefs>({ auto_save_interval_secs: 5 });
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  // AI state
  const [apiKey, setApiKey] = useState("");
  const [aiBusy, setAiBusy] = useState(false);
  const [aiMessage, setAiMessage] = useState("");
  const [consent, setConsent] = useState(() => localStorage.getItem("sindri_cloud_ai_consent") === "accepted");

  useEffect(() => {
    if (!open) return;
    setTab(initialTab);
    setAiMessage("");
    setApiKey("");
  }, [open, initialTab]);

  useEffect(() => {
    if (!open || !projectPath) return;
    invoke<ProjectSettings>("get_project_settings", { projectPath }).then(setProj).catch(() => {});
    invoke<EditorPrefs>("get_editor_prefs").then(setPrefs).catch(() => {});
    if (engineReady) {
      fetch("http://localhost:7878/scene/world").then(r => r.json()).then((w: WorldSettings) => setWorld(w)).catch(() => {});
    }
  }, [open, projectPath, engineReady]);

  if (!open) return null;

  const handleSave = async () => {
    if (!projectPath) return;
    setSaving(true);
    try {
      await invoke("save_project_settings", { projectPath, settings: proj });
      await invoke("save_editor_prefs", { prefs });
      if (engineReady) {
        await fetch("http://localhost:7878/scene/world", {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ gravity_x: world.gravity_x, gravity_y: world.gravity_y }),
        });
      }
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("Failed to save settings:", e);
    } finally {
      setSaving(false);
    }
  };

  // AI helpers
  const cloud = provider !== "ollama";
  const activeStatus = providerStatuses.find(s => s.provider === provider);

  const acceptConsent = () => { localStorage.setItem("sindri_cloud_ai_consent", "accepted"); setConsent(true); };

  const saveKey = async () => {
    if (!cloud) return;
    setAiBusy(true); setAiMessage("");
    try { await invoke("save_ai_api_key", { provider, apiKey }); setApiKey(""); await onRefreshProviders(); setAiMessage(`${provider} key saved to OS keychain.`); }
    catch (err) { setAiMessage(String(err)); }
    finally { setAiBusy(false); }
  };

  const clearKey = async () => {
    if (!cloud) return;
    setAiBusy(true); setAiMessage("");
    try { await invoke("clear_ai_api_key", { provider }); await onRefreshProviders(); setAiMessage(`${provider} key cleared.`); }
    catch (err) { setAiMessage(String(err)); }
    finally { setAiBusy(false); }
  };

  const testProvider = async () => {
    setAiBusy(true); setAiMessage("");
    try {
      await invoke("test_ai_provider", {
        provider,
        model: modelConfig.assistant ?? (provider === "ollama" ? preferredOllamaModel(ollamaModels, "assistant") : activeStatus?.defaultModel),
      });
      await onRefreshProviders();
      setAiMessage(`${provider} responded successfully.`);
    } catch (err) { setAiMessage(String(err)); }
    finally { setAiBusy(false); }
  };

  return createPortal(
    <div
      style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 9000 }}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div style={{ width: "780px", maxWidth: "calc(100vw - 48px)", maxHeight: "calc(100vh - 64px)", background: "var(--paper)", border: "1px solid var(--rule-2)", display: "flex", flexDirection: "column", boxShadow: "0 24px 80px rgba(0,0,0,0.5)" }}>

        {/* Header */}
        <div style={{ height: "48px", display: "flex", alignItems: "center", padding: "0 20px", borderBottom: "1px solid var(--rule)", flexShrink: 0, gap: "10px" }}>
          <span style={{ fontFamily: "var(--font-ui)", fontWeight: 600, fontSize: "14px", color: "var(--ink)", flex: 1 }}>Settings</span>
          <button onClick={onClose} style={{ width: "24px", height: "24px", background: "none", border: "none", color: "var(--ink-3)", fontSize: "18px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", lineHeight: 1 }}>×</button>
        </div>

        {/* Body: sidebar + content */}
        <div style={{ display: "flex", flex: 1, overflow: "hidden", minHeight: 0 }}>

          {/* Sidebar */}
          <div style={{ width: "160px", borderRight: "1px solid var(--rule)", padding: "12px 0", flexShrink: 0, display: "flex", flexDirection: "column", gap: "2px" }}>
            {TABS.map(t => (
              <button
                key={t.key}
                onClick={() => { setTab(t.key); setAiMessage(""); }}
                style={{
                  display: "flex", alignItems: "center", gap: "10px",
                  padding: "8px 16px",
                  background: tab === t.key ? "rgba(240,192,80,0.1)" : "none",
                  border: "none",
                  borderLeft: tab === t.key ? "2px solid var(--amber)" : "2px solid transparent",
                  color: tab === t.key ? "var(--ink)" : "var(--ink-3)",
                  fontFamily: "var(--font-ui)", fontSize: "13px",
                  cursor: "pointer", textAlign: "left", width: "100%",
                }}
              >
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "13px", color: tab === t.key ? "var(--amber)" : "var(--ink-4)", width: "16px", flexShrink: 0 }}>{t.icon}</span>
                {t.label}
              </button>
            ))}
          </div>

          {/* Content */}
          <div style={{ flex: 1, overflowY: "auto", padding: "24px", minWidth: 0 }}>

            {/* ── Project ─────────────────────────────────────────── */}
            {tab === "project" && (
              <div style={{ display: "grid", gap: "20px" }}>
                <Field label="Project Name">
                  <Input value={proj.name} onChange={v => setProj(p => ({ ...p, name: v }))} />
                </Field>

                <Field label="Resolution" hint="Takes effect after restarting the engine.">
                  <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                    <Input type="number" value={String(proj.resolution_width)} style={{ width: "72px" }}
                      onChange={v => setProj(p => ({ ...p, resolution_width: Math.max(1, parseInt(v) || 1280) }))} />
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-4)" }}>×</span>
                    <Input type="number" value={String(proj.resolution_height)} style={{ width: "72px" }}
                      onChange={v => setProj(p => ({ ...p, resolution_height: Math.max(1, parseInt(v) || 720) }))} />
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-4)" }}>px</span>
                  </div>
                </Field>

                <ToggleField
                  label="Pixel Art Mode"
                  detail="Nearest-neighbor filtering · pixel-perfect camera"
                  hint="Takes effect after restarting the engine."
                  value={proj.pixel_art_mode}
                  onChange={v => setProj(p => ({ ...p, pixel_art_mode: v }))}
                />

                <Field label="Gravity" hint="Applied to physics bodies · units/s²">
                  <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)", width: "10px" }}>x</span>
                    <Input type="number" value={String(world.gravity_x)} style={{ width: "96px" }}
                      onChange={v => setWorld(w => ({ ...w, gravity_x: parseFloat(v) || 0 }))} />
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)", width: "10px" }}>y</span>
                    <Input type="number" value={String(world.gravity_y)} style={{ width: "96px" }}
                      onChange={v => setWorld(w => ({ ...w, gravity_y: parseFloat(v) || 0 }))} />
                  </div>
                </Field>
              </div>
            )}

            {/* ── Editor ──────────────────────────────────────────── */}
            {tab === "editor" && (
              <div style={{ display: "grid", gap: "20px" }}>
                <Field label="Auto-save interval">
                  <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                    <Input type="number" value={String(prefs.auto_save_interval_secs)} style={{ width: "72px" }}
                      onChange={v => setPrefs(p => ({ ...p, auto_save_interval_secs: Math.max(1, parseInt(v) || 5) }))} />
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--ink-4)" }}>seconds</span>
                  </div>
                </Field>
              </div>
            )}

            {/* ── AI ──────────────────────────────────────────────── */}
            {tab === "ai" && (
              <div style={{ display: "grid", gap: "18px" }}>
                {/* Provider cards */}
                <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "8px" }}>
                  {(
                    [
                      { id: "ollama" as AiProvider,    title: "Ollama",    detail: "Offline local models on localhost. No API key and no external network calls." },
                      { id: "openai" as AiProvider,    title: "OpenAI",    detail: "BYOK cloud models for stronger code, vision, and reasoning." },
                      { id: "anthropic" as AiProvider, title: "Anthropic", detail: "BYOK Claude models for long-context planning and code review." },
                    ]
                  ).map(({ id, title, detail }) => {
                    const status = providerStatuses.find(s => s.provider === id);
                    const selected = provider === id;
                    return (
                      <button
                        key={id}
                        onClick={() => { setProvider(id); setAiMessage(""); }}
                        style={{
                          textAlign: "left", padding: "12px",
                          border: `1px solid ${selected ? "var(--amber)" : "var(--rule-2)"}`,
                          background: selected ? "rgba(240,192,80,0.08)" : "var(--paper-2)",
                          color: selected ? "var(--ink)" : "var(--ink-2)",
                          cursor: "pointer", display: "grid", gap: "5px", fontFamily: "var(--font-ui)",
                        }}
                      >
                        <span style={{ display: "flex", justifyContent: "space-between", gap: "8px" }}>
                          <strong style={{ fontSize: "13px" }}>{title}</strong>
                          <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: status?.configured ? "var(--moss)" : "var(--ink-4)" }}>
                            {status?.configured ? "configured" : id === "ollama" ? "offline" : "no key"}
                          </span>
                        </span>
                        <span style={{ color: "var(--ink-3)", fontSize: "11px", lineHeight: 1.4 }}>{detail}</span>
                      </button>
                    );
                  })}
                </div>

                {/* Cloud consent */}
                {cloud && (
                  <div style={{ border: `1px solid ${consent ? "var(--rule)" : "var(--amber)"}`, background: consent ? "var(--paper-2)" : "rgba(240,192,80,0.06)", padding: "14px", display: "grid", gap: "10px", fontSize: "12px", color: "var(--ink-2)", lineHeight: 1.5 }}>
                    <strong style={{ color: "var(--ink)", fontSize: "13px" }}>Cloud AI privacy notice</strong>
                    <p style={{ margin: 0 }}>
                      When using OpenAI or Anthropic, scene JSON, scripts, screenshots, errors, and your prompt may be sent to that provider.
                      Sindri is not responsible for data you choose to send. Use at your own discretion.
                    </p>
                    {!consent && (
                      <button onClick={acceptConsent} style={{ justifySelf: "start", background: "var(--amber)", color: "var(--paper)", border: "none", padding: "7px 12px", cursor: "pointer", fontFamily: "var(--font-ui)", fontSize: "12px" }}>
                        I understand and want cloud AI available
                      </button>
                    )}
                  </div>
                )}

                {/* Model roles + API key */}
                <div style={{ display: "grid", gridTemplateColumns: cloud ? "1fr 200px" : "1fr", gap: "20px", alignItems: "start" }}>
                  <div style={{ display: "grid", gap: "6px" }}>
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: "0.08em" }}>Model Roles</span>
                    {aiModelRoles.map(({ role, title, detail }) => (
                      <ModelRolePicker
                        key={role}
                        provider={provider}
                        role={role}
                        title={title}
                        detail={detail}
                        value={modelConfig[role]}
                        models={ollamaModels}
                        defaultModel={provider === "ollama" ? preferredOllamaModel(ollamaModels, role) : activeStatus?.defaultModel ?? ""}
                        onChange={m => setModelForRole(role, m)}
                      />
                    ))}
                    {provider === "ollama" && (
                      <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
                        {ollamaModels.length > 0 ? `from \`ollama list\` · ${ollamaModels.length} installed` : "No installed models found from `ollama list`."}
                      </span>
                    )}
                  </div>

                  {cloud && (
                    <label style={{ display: "grid", gap: "6px", opacity: consent ? 1 : 0.45 }}>
                      <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: "0.08em" }}>API Key</span>
                      <input
                        value={apiKey}
                        disabled={!consent}
                        onChange={e => setApiKey(e.target.value)}
                        type="password"
                        placeholder={activeStatus?.configured ? "saved in OS keychain" : `${provider.toUpperCase()} API key`}
                        style={{ background: "var(--paper-2)", border: "1px solid var(--rule-2)", color: "var(--ink)", padding: "7px 10px", fontFamily: "var(--font-mono)", fontSize: "12px", outline: "none", width: "100%", boxSizing: "border-box" }}
                      />
                    </label>
                  )}
                </div>

                {/* AI action buttons */}
                <div style={{ display: "flex", gap: "8px", alignItems: "center", flexWrap: "wrap" }}>
                  {cloud && (
                    <>
                      <AiBtn disabled={!consent || aiBusy || !apiKey.trim()} onClick={saveKey} primary>save key</AiBtn>
                      <AiBtn disabled={!consent || aiBusy || !activeStatus?.configured} onClick={clearKey}>clear key</AiBtn>
                    </>
                  )}
                  <AiBtn disabled={aiBusy || (cloud && (!consent || !activeStatus?.configured))} onClick={testProvider}>test provider</AiBtn>
                </div>

                {aiMessage && (
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: aiMessage.includes("success") || aiMessage.includes("saved") ? "var(--moss)" : "var(--ink-3)", borderTop: "1px solid var(--rule)", paddingTop: "12px" }}>
                    {aiMessage}
                  </span>
                )}
              </div>
            )}
          </div>
        </div>

        {/* Footer — only for project/editor (AI has its own inline actions) */}
        {tab !== "ai" && (
          <div style={{ height: "52px", borderTop: "1px solid var(--rule)", display: "flex", alignItems: "center", justifyContent: "flex-end", padding: "0 20px", gap: "10px", flexShrink: 0 }}>
            {saved && <span style={{ fontSize: "11px", color: "var(--moss)", fontFamily: "var(--font-mono)" }}>Saved</span>}
            <button onClick={onClose} style={footerBtnStyle(false)}>Cancel</button>
            <button onClick={handleSave} disabled={saving || !projectPath} style={footerBtnStyle(false, true, saving || !projectPath)}>
              {saving ? "Saving…" : "Save"}
            </button>
          </div>
        )}
        {tab === "ai" && (
          <div style={{ height: "52px", borderTop: "1px solid var(--rule)", display: "flex", alignItems: "center", justifyContent: "flex-end", padding: "0 20px" }}>
            <button onClick={onClose} style={footerBtnStyle(false, true)}>Done</button>
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}

// ─── Sub-components ───────────────────────────────────────────────────────────

function Field({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div>
      <label style={{ display: "block", fontFamily: "var(--font-ui)", fontSize: "13px", color: "var(--ink)", marginBottom: "6px", fontWeight: 500 }}>{label}</label>
      {children}
      {hint && <span style={{ display: "block", fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", marginTop: "5px" }}>{hint}</span>}
    </div>
  );
}

function Input({ value, onChange, type = "text", style: extraStyle }: { value: string; onChange?: (v: string) => void; type?: string; style?: CSSProperties }) {
  return (
    <input
      type={type}
      value={value}
      onChange={e => onChange?.(e.target.value)}
      style={{ height: "30px", padding: "0 8px", background: "var(--paper-2)", border: "1px solid var(--rule)", color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "12px", outline: "none", boxSizing: "border-box", width: "100%", ...extraStyle }}
    />
  );
}

function ToggleField({ label, detail, hint, value, onChange }: { label: string; detail?: string; hint?: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "16px" }}>
        <div>
          <span style={{ fontFamily: "var(--font-ui)", fontSize: "13px", color: "var(--ink)", fontWeight: 500 }}>{label}</span>
          {detail && <span style={{ display: "block", fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", marginTop: "2px" }}>{detail}</span>}
        </div>
        <button
          onClick={() => onChange(!value)}
          style={{ width: "36px", height: "20px", background: value ? "var(--amber)" : "var(--paper-3)", border: "1px solid var(--rule-2)", borderRadius: "10px", position: "relative", cursor: "pointer", flexShrink: 0, transition: "background 0.15s" }}
        >
          <span style={{ position: "absolute", top: "2px", left: value ? "17px" : "2px", width: "14px", height: "14px", background: "var(--ink)", borderRadius: "50%", transition: "left 0.15s" }} />
        </button>
      </div>
      {hint && <span style={{ display: "block", fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", marginTop: "5px" }}>{hint}</span>}
    </div>
  );
}

function AiBtn({ children, onClick, disabled, primary }: { children: React.ReactNode; onClick: () => void; disabled?: boolean; primary?: boolean }) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        background: disabled ? "transparent" : primary ? "var(--ink)" : "var(--paper-2)",
        border: "1px solid var(--rule-2)",
        color: disabled ? "var(--ink-4)" : primary ? "var(--paper)" : "var(--ink-2)",
        padding: "7px 12px", cursor: disabled ? "default" : "pointer",
        fontFamily: "var(--font-ui)", fontSize: "12px",
      }}
    >
      {children}
    </button>
  );
}

function footerBtnStyle(cancel: boolean, primary = false, disabled = false): CSSProperties {
  return {
    height: "30px", padding: "0 16px",
    background: disabled ? "var(--paper-2)" : primary ? "var(--amber)" : "none",
    border: primary ? "none" : "1px solid var(--rule)",
    color: disabled ? "var(--ink-4)" : primary ? "var(--paper)" : "var(--ink-3)",
    fontSize: "12px", fontFamily: "var(--font-ui)", fontWeight: primary ? 600 : 400,
    cursor: disabled ? "default" : "pointer",
    opacity: cancel ? 0.7 : 1,
  };
}

function ModelRolePicker({
  provider, role: _role, title, detail, value, models, defaultModel, onChange,
}: {
  provider: AiProvider; role: AiModelRole; title: string; detail: string;
  value: string | null; models: string[]; defaultModel: string | null;
  onChange: (model: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const [menuRect, setMenuRect] = useState<{ left: number; top: number; width: number } | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const local = provider === "ollama";
  const hasLocalModels = local && models.length > 0;
  const resolvedDefault = defaultModel || null;

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (!menuRef.current?.contains(e.target as Node) && !buttonRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const openMenu = () => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (rect) setMenuRect({ left: rect.left, top: rect.bottom + 4, width: rect.width });
    setOpen(true);
  };

  const choose = (m: string | null) => { onChange(m); setOpen(false); };

  const displayValue = value || resolvedDefault || (local ? "no model" : "");

  return (
    <div style={{ display: "grid", gridTemplateColumns: "100px 1fr", gap: "8px", alignItems: "start", padding: "8px 0", borderBottom: "1px solid var(--rule)" }}>
      <div>
        <div style={{ fontFamily: "var(--font-ui)", fontSize: "12px", color: "var(--ink)", fontWeight: 500 }}>{title}</div>
        <div style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)", lineHeight: 1.4, marginTop: "2px" }}>{detail}</div>
      </div>
      {local ? (
        <>
          <button
            ref={buttonRef}
            onClick={openMenu}
            disabled={!hasLocalModels}
            style={{ display: "flex", alignItems: "center", justifyContent: "space-between", height: "30px", padding: "0 10px", background: "var(--paper-2)", border: "1px solid var(--rule-2)", color: hasLocalModels ? "var(--ink)" : "var(--ink-4)", fontFamily: "var(--font-mono)", fontSize: "11px", cursor: hasLocalModels ? "pointer" : "default" }}
          >
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{displayValue}</span>
            <span style={{ color: "var(--ink-4)", flexShrink: 0 }}>▾</span>
          </button>
          {open && menuRect && createPortal(
            <div ref={menuRef} style={{ position: "fixed", left: menuRect.left, top: menuRect.top, width: menuRect.width, background: "var(--paper)", border: "1px solid var(--rule-2)", boxShadow: "0 8px 32px rgba(0,0,0,0.4)", zIndex: 99999, maxHeight: "240px", overflowY: "auto" }}>
              {resolvedDefault && resolvedDefault !== value && (
                <button onClick={() => choose(null)} style={{ display: "block", width: "100%", textAlign: "left", padding: "7px 10px", background: "none", border: "none", color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "11px", cursor: "pointer" }}>
                  {resolvedDefault} <span style={{ color: "var(--ink-4)" }}>(auto)</span>
                </button>
              )}
              {models.map(m => (
                <button key={m} onClick={() => choose(m)} style={{ display: "block", width: "100%", textAlign: "left", padding: "7px 10px", background: value === m ? "rgba(240,192,80,0.12)" : "none", border: "none", color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", cursor: "pointer" }}>
                  {m}
                </button>
              ))}
            </div>,
            document.body,
          )}
        </>
      ) : (
        <input
          type="text"
          value={value ?? ""}
          onChange={e => onChange(e.target.value || null)}
          placeholder={resolvedDefault ?? `Enter ${provider} model`}
          style={{ height: "30px", padding: "0 10px", background: "var(--paper-2)", border: "1px solid var(--rule-2)", color: "var(--ink)", fontFamily: "var(--font-mono)", fontSize: "11px", outline: "none" }}
        />
      )}
    </div>
  );
}
