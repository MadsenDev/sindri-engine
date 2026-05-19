import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Scene, Entity, ProposalData } from "../App";
import { KbdKey } from "../App";

interface Message {
  role: "user" | "ai";
  text: string;
  hadChanges?: boolean;
}

interface ProjectFile {
  path: string;
  kind: string;
  name: string;
}

interface Props {
  scene: Scene | null;
  projectPath: string;
  openScript: { path: string; content: string } | null;
  selectedModel: string | null;
  projectFiles: ProjectFile[];
  runtimeErrors: string[];
  onSceneChange: () => void;
  onClose: () => void;
  selectedEntity: Entity | null;
  onProposalReady?: (proposal: ProposalData) => void;
  initialMessage?: string;
  initialInput?: string;
}

type ContextFlag = "scene" | "script" | "viewport" | "errors";

function chatHistoryKey(projectPath: string) {
  return `sindri_chat_history:${projectPath}`;
}

function loadMessages(projectPath: string): Message[] {
  try {
    const saved = localStorage.getItem(chatHistoryKey(projectPath));
    if (!saved) return [];
    return JSON.parse(saved) as Message[];
  } catch { return []; }
}

export default function CmdK({
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  scene, projectPath, openScript, selectedModel,
  projectFiles: _projectFiles, runtimeErrors: _runtimeErrors,
  onSceneChange: _onSceneChange, onClose, selectedEntity,
  onProposalReady, initialMessage, initialInput,
}: Props) {
  const [messages, setMessages] = useState<Message[]>(() => loadMessages(projectPath));
  const [input, setInput] = useState(initialInput ?? "");
  const [thinking, setThinking] = useState(false);
  const [activeFlags] = useState<Set<ContextFlag>>(new Set(["scene"]));
  const inputRef = useRef<HTMLInputElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const autoSentRef = useRef(false);

  useEffect(() => {
    inputRef.current?.focus();
    // Move cursor to end of pre-filled text
    const el = inputRef.current;
    if (el && initialInput) {
      el.setSelectionRange(el.value.length, el.value.length);
    }
  }, []);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages, thinking]);

  const callAI = async (message: string, currentMessages: Message[]): Promise<Message[]> => {
    const history = currentMessages.flatMap(m => {
      if (m.role === "user") return [{ role: "user", content: m.text }];
      if (m.role === "ai") return [{ role: "assistant", content: m.text }];
      return [];
    });

    const proposal = await invoke<ProposalData>("generate_proposal", {
      message,
      contextFlags: {
        includeScene: activeFlags.has("scene"),
        includeScript: activeFlags.has("script"),
        includeViewport: activeFlags.has("viewport"),
        includeErrors: activeFlags.has("errors"),
      },
      model: selectedModel ?? undefined,
      history,
      openScript: activeFlags.has("script") ? openScript : null,
    });

    const displayText = proposal.summary || "(no summary)";
    const hadChanges = proposal.changes.length > 0;
    const aiMsg: Message = { role: "ai", text: displayText, hadChanges };
    const withAI = [...currentMessages, aiMsg];
    setMessages(withAI);

    if (hadChanges && onProposalReady) {
      onProposalReady(proposal);
    }

    return withAI;
  };

  const sendMessage = async (textOverride?: string) => {
    const text = (textOverride ?? input).trim();
    if (!text || thinking) return;
    setInput("");

    const userMsg: Message = { role: "user", text };
    const nextMessages = [...messages, userMsg];
    setMessages(nextMessages);
    setThinking(true);

    try {
      const final = await callAI(text, nextMessages);
      localStorage.setItem(chatHistoryKey(projectPath), JSON.stringify(final.slice(-40)));
    } catch (err) {
      setMessages(prev => [...prev, { role: "ai", text: `Error: ${String(err)}` }]);
    } finally {
      setThinking(false);
    }
  };

  useEffect(() => {
    if (initialMessage && !autoSentRef.current) {
      autoSentRef.current = true;
      sendMessage(initialMessage);
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const ctxChips = [
    { kind: "scene", text: scene?.name ?? "scene" },
    ...(selectedEntity ? [{ kind: "entity", text: `@${selectedEntity.name}` }] : []),
  ];

  return (
    <div
      style={{
        position: "fixed", inset: 0,
        background: "rgba(0,0,0,0.6)",
        zIndex: 200,
        display: "flex", alignItems: "flex-start", justifyContent: "center",
        paddingTop: "120px",
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: "680px",
          background: "var(--paper)",
          border: "1px solid var(--rule-2)",
          boxShadow: "0 24px 60px rgba(0,0,0,0.35)",
          display: "flex", flexDirection: "column",
          maxHeight: "calc(100vh - 180px)",
        }}
        onClick={e => e.stopPropagation()}
      >
        {/* Input row */}
        <div style={{
          display: "flex", alignItems: "center", gap: "12px",
          padding: "18px 22px 16px",
          borderBottom: "1px solid var(--rule)",
        }}>
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none">
            <path d="M8 1v14M1 8h14M3.5 3.5l9 9M12.5 3.5l-9 9" stroke="var(--amber)" strokeWidth="1.4" strokeLinecap="square" />
          </svg>
          <input
            ref={inputRef}
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => {
              if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendMessage(); }
              if (e.key === "Escape") onClose();
            }}
            placeholder={`Ask Sindri${selectedEntity ? ` about ${selectedEntity.name}` : " to build something"}…`}
            style={{
              flex: 1,
              background: "none", border: "none", outline: "none",
              fontFamily: "var(--font-ui)", fontSize: "22px",
              color: "var(--ink)",
              lineHeight: 1.1,
            }}
          />
          <KbdKey>esc</KbdKey>
        </div>

        {/* Context chips */}
        <div style={{
          padding: "10px 22px",
          borderBottom: "1px solid var(--rule)",
          display: "flex", gap: "8px", alignItems: "center", flexWrap: "wrap",
        }}>
          <span style={{ fontSize: "11.5px", color: "var(--ink-4)", fontFamily: "var(--font-mono)" }}>Context:</span>
          {ctxChips.map((c, i) => (
            <span key={i} style={{
              fontSize: "11px", fontFamily: "var(--font-mono)",
              padding: "3px 8px",
              border: `1px solid ${c.kind === "entity" ? "var(--cyan)" : "var(--rule-2)"}`,
              color: c.kind === "entity" ? "var(--cyan)" : "var(--ink-3)",
              background: c.kind === "entity" ? "rgba(109,188,219,0.10)" : "transparent",
              display: "inline-flex", gap: "6px", alignItems: "center",
            }}>
              {c.kind === "entity" && <span style={{ width: "5px", height: "5px", background: "var(--cyan)", display: "inline-block" }} />}
              {c.text}
            </span>
          ))}
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>
            + type @ to add context
          </span>
        </div>

        {/* Message history */}
        {messages.length > 0 && (
          <div
            ref={scrollRef}
            style={{
              flex: 1, overflowY: "auto",
              padding: "8px 0",
              maxHeight: "360px",
            }}
          >
            {messages.map((msg, i) => (
              <div key={i} style={{
                padding: "10px 22px",
                borderBottom: "1px solid var(--rule)",
              }}>
                <span style={{
                  fontSize: "10px", color: "var(--ink-4)",
                  fontFamily: "var(--font-mono)",
                  letterSpacing: "0.05em",
                  display: "block", marginBottom: "4px",
                }}>
                  {msg.role === "user" ? "you" : "✦ sindri"}
                </span>
                <div style={{
                  fontSize: "13px", color: msg.role === "ai" ? "var(--ink)" : "var(--ink-2)",
                  fontFamily: "var(--font-ui)", lineHeight: 1.55,
                  whiteSpace: "pre-wrap", wordBreak: "break-word",
                }}>
                  {msg.text}
                </div>
                {msg.role === "ai" && msg.hadChanges && (
                  <div style={{ marginTop: "8px" }}>
                    <span style={{
                      fontFamily: "var(--font-mono)", fontSize: "10.5px",
                      padding: "2px 8px",
                      border: "1px solid var(--amber)",
                      color: "var(--amber)",
                    }}>
                      ✦ proposal staged → review in inspector
                    </span>
                  </div>
                )}
              </div>
            ))}
            {thinking && (
              <div style={{ padding: "14px 22px", display: "flex", alignItems: "center", gap: "10px" }}>
                <span style={{ fontSize: "10px", color: "var(--ink-4)", fontFamily: "var(--font-mono)" }}>✦ sindri</span>
                <span style={{ width: "6px", height: "6px", background: "var(--amber)", animation: "v3dot 900ms 0ms ease-in-out infinite" }} />
                <span style={{ width: "6px", height: "6px", background: "var(--amber)", animation: "v3dot 900ms 140ms ease-in-out infinite" }} />
                <span style={{ width: "6px", height: "6px", background: "var(--amber)", animation: "v3dot 900ms 280ms ease-in-out infinite" }} />
              </div>
            )}
          </div>
        )}

        {/* Footer */}
        <div style={{
          borderTop: "1px solid var(--rule)",
          padding: "10px 22px",
          display: "flex", alignItems: "center", justifyContent: "space-between",
          fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)",
        }}>
          <div style={{ display: "flex", gap: "16px" }}>
            <span><KbdKey>↵</KbdKey> send</span>
            <span><KbdKey>esc</KbdKey> close</span>
            {messages.length > 0 && (
              <button
                onClick={() => { setMessages([]); localStorage.removeItem(chatHistoryKey(projectPath)); }}
                style={{
                  background: "none", border: "none", color: "var(--ink-4)",
                  cursor: "pointer", fontFamily: "var(--font-mono)", fontSize: "11px",
                  padding: 0,
                }}
              >clear history</button>
            )}
          </div>
          <div style={{ display: "inline-flex", alignItems: "center", gap: "6px" }}>
            <span style={{ width: "6px", height: "6px", background: selectedModel ? "var(--moss)" : "var(--ink-4)", display: "inline-block" }} />
            <span>{selectedModel ?? "no model"} · local</span>
          </div>
        </div>
      </div>
    </div>
  );
}
