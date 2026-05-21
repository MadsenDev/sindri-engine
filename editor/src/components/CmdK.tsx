import { useState, useEffect, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Scene, Entity, ProposalData, AiModelConfig, AiModelRole } from "../App";
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
  modelConfig: AiModelConfig;
  selectedProvider: "ollama" | "openai" | "anthropic" | "openrouter";
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
  scene, projectPath, openScript, modelConfig, selectedProvider,
  projectFiles, runtimeErrors: _runtimeErrors,
  onSceneChange: _onSceneChange, onClose, selectedEntity,
  onProposalReady, initialMessage, initialInput,
}: Props) {
  const [messages, setMessages] = useState<Message[]>(() => loadMessages(projectPath));
  const [input, setInput] = useState(initialInput ?? "");
  const [thinking, setThinking] = useState(false);
  const [activeFlags] = useState<Set<ContextFlag>>(new Set(["scene"]));

  // Mention autocomplete state
  const [mentionType, setMentionType] = useState<"@" | "#" | null>(null);
  const [mentionQuery, setMentionQuery] = useState("");
  const [mentionStart, setMentionStart] = useState(-1);
  const [mentionIndex, setMentionIndex] = useState(0);

  const inputRef = useRef<HTMLInputElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const autoSentRef = useRef(false);

  useEffect(() => {
    inputRef.current?.focus();
    const el = inputRef.current;
    if (el && initialInput) {
      el.setSelectionRange(el.value.length, el.value.length);
    }
  }, []);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages, thinking]);

  // Compute autocomplete suggestions
  const mentionSuggestions = useMemo(() => {
    if (mentionType === "@") {
      const entities = scene ? Object.values(scene.entities) : [];
      return entities
        .map(e => e.name)
        .filter(name => name.toLowerCase().startsWith(mentionQuery.toLowerCase()));
    }
    if (mentionType === "#") {
      const q = mentionQuery.toLowerCase();
      return projectFiles
        .filter(f => f.kind === "script" || f.path.endsWith(".lua"))
        .map(f => f.path)
        .filter(p => q === "" || p.toLowerCase().includes(q));
    }
    return [];
  }, [mentionType, mentionQuery, scene, projectFiles]);

  // Entities mentioned via @Name in the input
  const mentionedEntities = useMemo(() => {
    if (!scene) return [];
    const matches = [...input.matchAll(/@([\w]+)/g)];
    return matches
      .map(m => Object.values(scene.entities).find(e => e.name === m[1]))
      .filter((e): e is Entity => e !== undefined)
      .filter(e => e.id !== selectedEntity?.id);
  }, [input, scene, selectedEntity]);

  // Files mentioned via #path in the input
  const mentionedFiles = useMemo(() => {
    const matches = [...input.matchAll(/#([^\s]+)/g)];
    return matches.map(m => m[1]);
  }, [input]);

  const detectMention = (val: string, cursor: number) => {
    let i = cursor - 1;
    while (i >= 0 && val[i] !== " " && val[i] !== "\n" && val[i] !== "@" && val[i] !== "#") i--;
    if (i >= 0 && (val[i] === "@" || val[i] === "#")) {
      const query = val.slice(i + 1, cursor);
      if (!query.includes(" ")) {
        setMentionType(val[i] as "@" | "#");
        setMentionQuery(query);
        setMentionStart(i);
        setMentionIndex(0);
        return;
      }
    }
    setMentionType(null);
    setMentionQuery("");
    setMentionStart(-1);
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setInput(val);
    detectMention(val, e.target.selectionStart ?? val.length);
  };

  const insertMention = (value: string) => {
    const before = input.slice(0, mentionStart);
    const after = input.slice(mentionStart + 1 + mentionQuery.length);
    const tag = mentionType + value;
    const newInput = before + tag + " " + after.trimStart();
    setInput(newInput);
    setMentionType(null);
    setMentionQuery("");
    setMentionStart(-1);
    setTimeout(() => {
      if (inputRef.current) {
        const pos = before.length + tag.length + 1;
        inputRef.current.focus();
        inputRef.current.setSelectionRange(pos, pos);
      }
    }, 0);
  };

  const callAI = async (message: string, currentMessages: Message[]): Promise<Message[]> => {
    const scriptContext = await resolveScriptContext(message, scene, selectedEntity, openScript, activeFlags);
    const includeScript = activeFlags.has("script") || scriptContext !== null;
    const selectedRole = selectModelRole(message, activeFlags, scriptContext !== null);
    const selectedModel = modelConfig[selectedRole] ?? modelConfig.assistant ?? undefined;
    const history = currentMessages.flatMap(m => {
      if (m.role === "user") return [{ role: "user", content: m.text }];
      if (m.role === "ai") return [{ role: "assistant", content: m.text }];
      return [];
    });

    const proposal = await invoke<ProposalData>("generate_proposal", {
      message,
      contextFlags: {
        includeScene: activeFlags.has("scene"),
        includeScript,
        includeViewport: activeFlags.has("viewport"),
        includeErrors: activeFlags.has("errors"),
      },
      provider: selectedProvider,
      model: selectedModel ?? undefined,
      history,
      openScript: scriptContext,
    });

    const hadChanges = proposal.changes.length > 0;
    const displayText = proposal.summary?.trim()
      || (hadChanges
        ? `Staged ${proposal.changes.length} proposed change${proposal.changes.length === 1 ? "" : "s"}. Review in inspector.`
        : "No changes proposed.");
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
    setMentionType(null);

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
    ...mentionedEntities.map(e => ({ kind: "entity", text: `@${e.name}` })),
    ...mentionedFiles.map(f => ({ kind: "file", text: `#${f.split("/").pop()}` })),
  ];

  const dropdownOpen = mentionType !== null && mentionSuggestions.length > 0;

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
        <div style={{ position: "relative" }}>
          <div style={{
            display: "flex", alignItems: "center", gap: "12px",
            padding: "18px 22px 16px",
            borderBottom: dropdownOpen ? "none" : "1px solid var(--rule)",
          }}>
            <svg width="18" height="18" viewBox="0 0 16 16" fill="none">
              <path d="M8 1v14M1 8h14M3.5 3.5l9 9M12.5 3.5l-9 9" stroke="var(--amber)" strokeWidth="1.4" strokeLinecap="square" />
            </svg>
            <input
              ref={inputRef}
              value={input}
              onChange={handleInputChange}
              onKeyDown={e => {
                if (dropdownOpen) {
                  if (e.key === "ArrowDown") {
                    e.preventDefault();
                    setMentionIndex(i => Math.min(i + 1, mentionSuggestions.length - 1));
                  } else if (e.key === "ArrowUp") {
                    e.preventDefault();
                    setMentionIndex(i => Math.max(i - 1, 0));
                  } else if (e.key === "Enter") {
                    e.preventDefault();
                    insertMention(mentionSuggestions[mentionIndex]);
                  } else if (e.key === "Escape") {
                    e.preventDefault();
                    setMentionType(null);
                  }
                } else {
                  if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendMessage(); }
                  if (e.key === "Escape") onClose();
                }
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

          {/* Autocomplete dropdown */}
          {dropdownOpen && (
            <div style={{
              position: "absolute", left: 0, right: 0, top: "100%",
              background: "var(--paper)",
              border: "1px solid var(--rule-2)",
              borderTop: "1px solid var(--rule)",
              zIndex: 10,
              maxHeight: "180px", overflowY: "auto",
            }}>
              {mentionSuggestions.map((s, i) => (
                <div
                  key={s}
                  onMouseDown={e => { e.preventDefault(); insertMention(s); }}
                  style={{
                    padding: "8px 22px",
                    fontFamily: "var(--font-mono)", fontSize: "12px",
                    color: i === mentionIndex ? "var(--ink)" : "var(--ink-3)",
                    background: i === mentionIndex ? "var(--paper-2, rgba(255,255,255,0.04))" : "transparent",
                    cursor: "pointer",
                    display: "flex", alignItems: "center", gap: "8px",
                    borderBottom: i < mentionSuggestions.length - 1 ? "1px solid var(--rule)" : "none",
                  }}
                  onMouseEnter={() => setMentionIndex(i)}
                >
                  <span style={{
                    color: mentionType === "@" ? "var(--cyan)" : "var(--amber)",
                    fontSize: "10px",
                  }}>
                    {mentionType === "@" ? "entity" : "file"}
                  </span>
                  <span>{mentionType}{s}</span>
                </div>
              ))}
            </div>
          )}
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
              border: `1px solid ${c.kind === "entity" ? "var(--cyan)" : c.kind === "file" ? "var(--amber)" : "var(--rule-2)"}`,
              color: c.kind === "entity" ? "var(--cyan)" : c.kind === "file" ? "var(--amber)" : "var(--ink-3)",
              background: c.kind === "entity" ? "rgba(109,188,219,0.10)" : c.kind === "file" ? "rgba(255,176,0,0.08)" : "transparent",
              display: "inline-flex", gap: "6px", alignItems: "center",
            }}>
              {c.kind === "entity" && <span style={{ width: "5px", height: "5px", background: "var(--cyan)", display: "inline-block" }} />}
              {c.kind === "file" && <span style={{ width: "5px", height: "5px", background: "var(--amber)", display: "inline-block" }} />}
              {c.text}
            </span>
          ))}
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--ink-4)" }}>
            @ entity · # file
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
            <span style={{ width: "6px", height: "6px", background: modelConfig.assistant ? "var(--moss)" : "var(--ink-4)", display: "inline-block" }} />
            <span>{modelConfig.assistant ?? "default model"} · {selectedProvider}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function selectModelRole(
  message: string,
  activeFlags: Set<ContextFlag>,
  hasScriptContext: boolean,
): AiModelRole {
  const lower = message.toLowerCase();
  if (activeFlags.has("viewport")) return "vision";
  if (
    hasScriptContext
    || lower.includes("jump")
    || lower.includes("movement")
    || activeFlags.has("script")
    || lower.includes(".lua")
    || lower.includes("script")
    || lower.includes("code")
    || lower.includes("function")
    || lower.includes("api")
  ) {
    return "code";
  }
  return "assistant";
}

async function resolveScriptContext(
  message: string,
  scene: Scene | null,
  selectedEntity: Entity | null,
  openScript: { path: string; content: string } | null,
  activeFlags: Set<ContextFlag>,
): Promise<{ path: string; content: string } | null> {
  if (activeFlags.has("script") && openScript) return openScript;

  const path = findRelevantScriptPath(message, scene, selectedEntity);
  if (!path) return openScript && message.toLowerCase().includes("script") ? openScript : null;

  if (openScript?.path === path) return openScript;

  try {
    const content = await invoke<string>("get_script", { path });
    return { path, content };
  } catch {
    return { path, content: "" };
  }
}

function findRelevantScriptPath(
  message: string,
  scene: Scene | null,
  selectedEntity: Entity | null,
): string | null {
  const direct = message.match(/#([^\s]+\.lua)\b/);
  if (direct) return direct[1];

  const lower = message.toLowerCase();
  const entities = scene ? Object.values(scene.entities) : [];
  const referenced = entities.find(entity => lower.includes(entity.name.toLowerCase()));
  const target = referenced ?? selectedEntity;
  const script = target?.components.find(component => component.type === "Script");
  if (script && "path" in script && typeof script.path === "string" && script.path) {
    return script.path;
  }

  return null;
}
