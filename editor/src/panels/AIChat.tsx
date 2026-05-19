import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Scene, AiAction } from "../App";

type ActionStatus = "pending" | "ok" | "failed" | "suggestion";

interface Message {
  role: "user" | "ai";
  text: string;
  actions?: AiAction[];
  applied?: ActionStatus[];
  isErrorReport?: boolean; // auto-generated error report messages
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
  onSceneChange: () => void;
}

type ContextFlag = "scene" | "script" | "viewport" | "errors";

interface MentionCandidate {
  label: string;
  insert: string;
  hint?: string;    // secondary info shown in the popup
}

function chatHistoryKey(projectPath: string) {
  return `sindri_chat_history:${projectPath}`;
}

function loadMessages(projectPath: string): Message[] {
  try {
    const saved = localStorage.getItem(chatHistoryKey(projectPath));
    if (!saved) return [];
    const parsed = JSON.parse(saved) as Message[];
    // Migrate old boolean applied[] to ActionStatus[] (pre-ActionStatus format)
    return parsed.map(m => ({
      ...m,
      applied: m.applied?.map(a =>
        typeof a === "boolean" ? (a ? "ok" : "pending") : a as ActionStatus
      ),
    }));
  } catch { return []; }
}

export default function AIChat({ scene, projectPath, openScript: _openScript, selectedModel, projectFiles, onSceneChange }: Props) {
  const [messages, setMessages] = useState<Message[]>(() => loadMessages(projectPath));
  const [input, setInput] = useState("");
  const [thinking, setThinking] = useState(false);
  const [activeFlags, setActiveFlags] = useState<Set<ContextFlag>>(new Set(["scene"]));
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Mention picker state — tracks which sigil triggered it
  const [mentionQuery, setMentionQuery] = useState<string | null>(null); // null = closed
  const [mentionSigil, setMentionSigil] = useState<"@" | "#">("@");
  const [mentionIdx, setMentionIdx] = useState(0);

  useEffect(() => {
    setMessages(loadMessages(projectPath));
  }, [projectPath]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, thinking]);

  // @ candidates: entities (and their component types as sub-targets)
  const atCandidates: MentionCandidate[] = scene
    ? Object.values(scene.entities).flatMap(e => [
        { label: e.name, insert: `@${e.name}(id:${e.id})`, hint: `entity #${e.id}` },
        ...e.components.map(c => ({
          label: `${e.name}.${c.type}`,
          insert: `@${e.name}.${c.type}(id:${e.id})`,
          hint: `${c.type} on #${e.id}`,
        })),
      ])
    : [];

  // # candidates: project files
  const hashCandidates: MentionCandidate[] = projectFiles.map(f => ({
    label: f.name,
    insert: `#${f.path}`,
    hint: f.kind,
  }));

  const activeCandidates = mentionSigil === "@" ? atCandidates : hashCandidates;

  const filteredCandidates = mentionQuery !== null
    ? activeCandidates.filter(c =>
        c.label.toLowerCase().includes(mentionQuery.toLowerCase()) ||
        (c.hint ?? "").toLowerCase().includes(mentionQuery.toLowerCase())
      )
    : [];

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);

    const cursor = e.target.selectionStart ?? val.length;
    const before = val.slice(0, cursor);

    for (const sigil of ["@", "#"] as const) {
      const sigIdx = before.lastIndexOf(sigil);
      if (sigIdx !== -1) {
        const fragment = before.slice(sigIdx + 1);
        if (!fragment.includes(" ") && !fragment.includes("\n")) {
          setMentionSigil(sigil);
          setMentionQuery(fragment);
          setMentionIdx(0);
          return;
        }
      }
    }
    setMentionQuery(null);
  };

  const insertMention = (candidate: MentionCandidate) => {
    const ta = textareaRef.current;
    if (!ta) return;
    const cursor = ta.selectionStart ?? input.length;
    const before = input.slice(0, cursor);
    const after = input.slice(cursor);
    const sigIdx = before.lastIndexOf(mentionSigil);
    const newInput = before.slice(0, sigIdx) + candidate.insert + " " + after;
    setInput(newInput);
    setMentionQuery(null);
    setTimeout(() => {
      ta.focus();
      const newCursor = sigIdx + candidate.insert.length + 1;
      ta.setSelectionRange(newCursor, newCursor);
    }, 0);
  };

  const toggleFlag = (flag: ContextFlag) => {
    setActiveFlags(prev => {
      const next = new Set(prev);
      if (next.has(flag)) next.delete(flag);
      else next.add(flag);
      return next;
    });
  };

  // Run a list of actions, update their status live, return list of failure descriptions.
  const applyActions = async (
    actions: AiAction[],
    updateStatuses: (statuses: ActionStatus[]) => void,
  ): Promise<string[]> => {
    const statuses: ActionStatus[] = actions.map(a =>
      a.type === "suggest_fix" ? "suggestion" : "pending"
    );
    updateStatuses([...statuses]);

    const failures: string[] = [];
    for (let i = 0; i < actions.length; i++) {
      const action = actions[i];
      if (action.type === "suggest_fix") continue;
      try {
        await invoke("apply_action", { action });
        onSceneChange();
        statuses[i] = "ok";
      } catch (err) {
        statuses[i] = "failed";
        const a = action as Record<string, unknown>;
        const who = a.entity_name
          ? `"${a.entity_name}"`
          : a.entity_id != null
            ? `entity #${a.entity_id}`
            : "";
        failures.push(`${action.type}${who ? ` on ${who}` : ""}: ${String(err)}`);
      }
      updateStatuses([...statuses]);
    }
    return failures;
  };

  // Call the AI and apply its actions. If isRetry=true, failures won't recurse.
  const callAI = async (
    message: string,
    currentMessages: Message[],
    isRetry: boolean,
  ): Promise<Message[]> => {
    const history = currentMessages.flatMap(m => {
      if (m.role === "user" && !m.isErrorReport) return [{ role: "user", content: m.text }];
      if (m.role === "ai")                        return [{ role: "assistant", content: m.text }];
      return [];
    });

    const resp = await invoke<{ text: string; actions: AiAction[] }>("send_ai_message", {
      message,
      contextFlags: {
        includeScene: activeFlags.has("scene"),
        includeScript: activeFlags.has("script"),
        includeViewport: activeFlags.has("viewport"),
        includeErrors: activeFlags.has("errors"),
      },
      model: selectedModel ?? undefined,
      history,
    });

    const aiMsg: Message = {
      role: "ai",
      text: resp.text,
      actions: resp.actions,
      applied: resp.actions.map(a =>
        a.type === "suggest_fix" ? "suggestion" : "pending"
      ) as ActionStatus[],
    };
    const withAI = [...currentMessages, aiMsg];
    setMessages(withAI);

    // Apply actions, updating the last message's statuses live
    const failures = await applyActions(resp.actions, statuses => {
      setMessages(prev => {
        const updated = [...prev];
        const last = updated[updated.length - 1];
        if (last?.role === "ai") updated[updated.length - 1] = { ...last, applied: statuses };
        return updated;
      });
    });

    if (failures.length > 0 && !isRetry) {
      // Inject a visible error report, then ask the AI to fix it
      const errText =
        `⚠ ${failures.length} action${failures.length > 1 ? "s" : ""} failed:\n` +
        failures.map(f => `• ${f}`).join("\n");
      const errMsg: Message = { role: "ai", text: errText, isErrorReport: true };
      const withErr = [...withAI, errMsg];
      setMessages(withErr);

      const retryPrompt =
        `The following actions just failed — please try a different approach to accomplish the same goal:\n` +
        failures.map(f => `• ${f}`).join("\n");
      return await callAI(retryPrompt, withErr, true);
    }

    return withAI;
  };

  const sendMessage = async () => {
    const text = input.trim();
    if (!text || thinking) return;
    setInput("");

    const userMsg: Message = { role: "user", text };
    const nextMessages = [...messages, userMsg];
    setMessages(nextMessages);
    setThinking(true);

    try {
      const finalMessages = await callAI(text, nextMessages, false);
      localStorage.setItem(chatHistoryKey(projectPath), JSON.stringify(finalMessages.slice(-40)));
    } catch (err) {
      setMessages(prev => [...prev, { role: "ai", text: `Error: ${String(err)}` }]);
    } finally {
      setThinking(false);
    }
  };

  const flags: ContextFlag[] = ["scene", "script", "viewport", "errors"];

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden", minHeight: 0 }}>
      {/* AI header */}
      <div style={{
        height: "38px",
        borderBottom: "1px solid var(--border)",
        display: "flex",
        alignItems: "center",
        padding: "0 10px",
        gap: "8px",
        flexShrink: 0,
      }}>
        <span style={{
          fontFamily: "var(--font-ui)",
          fontWeight: 700,
          fontSize: "12px",
          color: "var(--text-bright)",
        }}>AI Assistant</span>
        <div style={{ flex: 1 }} />
        {messages.length > 0 && (
          <button
            onClick={() => {
              setMessages([]);
              localStorage.removeItem(chatHistoryKey(projectPath));
            }}
            title="Clear history"
            style={{
              background: "none", border: "none", color: "var(--text-dim)",
              cursor: "pointer", fontSize: "11px", padding: "2px 4px",
              borderRadius: "var(--radius)",
            }}
            onMouseEnter={e => (e.currentTarget as HTMLElement).style.color = "var(--text-muted)"}
            onMouseLeave={e => (e.currentTarget as HTMLElement).style.color = "var(--text-dim)"}
          >✕</button>
        )}
        {selectedModel && (
          <div style={{
            background: "var(--ai-glow)",
            border: "1px solid var(--ai-dim)",
            borderRadius: "var(--radius)",
            padding: "2px 7px",
            fontSize: "10px",
            color: "var(--ai)",
            maxWidth: "140px",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}>{selectedModel}</div>
        )}
      </div>

      {/* Context bar */}
      <div style={{
        height: "28px",
        borderBottom: "1px solid var(--border)",
        display: "flex",
        alignItems: "center",
        padding: "0 8px",
        gap: "4px",
        flexShrink: 0,
      }}>
        {flags.map(flag => {
          const active = activeFlags.has(flag);
          return (
            <button key={flag} onClick={() => toggleFlag(flag)} style={{
              background: active ? "var(--accent-glow)" : "var(--bg-3)",
              border: `1px solid ${active ? "var(--accent-dim)" : "var(--border)"}`,
              borderRadius: "var(--radius)",
              color: active ? "var(--accent)" : "var(--text-muted)",
              fontFamily: "var(--font-mono)",
              fontSize: "10px",
              padding: "2px 7px",
              cursor: "pointer",
            }}>{flag}</button>
          );
        })}
      </div>

      {/* Messages */}
      <div style={{ flex: 1, overflow: "auto", padding: "8px", display: "flex", flexDirection: "column", gap: "8px" }}>
        {messages.map((msg, i) => (
          <div key={i} style={{ display: "flex", flexDirection: "column", alignItems: msg.role === "user" ? "flex-end" : "flex-start" }}>
            <span style={{
              fontSize: "9px",
              color: "var(--text-dim)",
              letterSpacing: "0.05em",
              marginBottom: "3px",
            }}>{msg.role === "user" ? "you" : "ai"}</span>

            <div style={{
              maxWidth: "92%",
              padding: "7px 10px",
              background: msg.role === "user" ? "var(--accent-glow)" : "var(--ai-glow)",
              border: `1px solid ${msg.role === "user" ? "var(--accent-dim)" : "var(--ai-dim)"}`,
              borderRadius: msg.role === "user" ? "8px 8px 8px 2px" : "2px 8px 8px 8px",
              fontSize: "11px",
              color: "var(--text-bright)",
              lineHeight: 1.6,
              wordBreak: "break-word",
            }}>
              {msg.role === "ai" ? <Markdown text={msg.text} /> : msg.text}
            </div>

            {msg.actions && msg.actions.length > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "4px", marginTop: "4px", width: "92%" }}>
                {msg.actions.map((action, ai) => (
                  <ActionCard
                    key={ai}
                    action={action}
                    status={msg.applied?.[ai] ?? "pending"}
                    onApply={() => {
                      invoke("apply_action", { action }).then(() => onSceneChange()).catch(console.error);
                    }}
                  />
                ))}
              </div>
            )}
          </div>
        ))}

        {thinking && (
          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
            <span style={{ fontSize: "9px", color: "var(--text-dim)", letterSpacing: "0.05em" }}>ai</span>
            <div style={{ display: "flex", gap: "3px", alignItems: "center" }}>
              {[0, 1, 2].map(d => (
                <span key={d} style={{
                  width: "5px",
                  height: "5px",
                  borderRadius: "50%",
                  background: "var(--ai)",
                  display: "inline-block",
                  animation: `bounce 0.8s ${d * 0.15}s infinite`,
                }} />
              ))}
              <span style={{ fontSize: "10px", color: "var(--text-muted)", marginLeft: "4px" }}>analyzing…</span>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Chat input */}
      <div style={{
        background: "var(--bg-0)",
        borderTop: "1px solid var(--border)",
        padding: "8px",
        flexShrink: 0,
        position: "relative",
      }}>
        {/* Mention popup (@entities / #files) */}
        {mentionQuery !== null && filteredCandidates.length > 0 && (
          <div style={{
            position: "absolute", bottom: "calc(100% - 4px)", left: "8px", right: "8px",
            background: "var(--bg-2)", border: "1px solid var(--border-bright)",
            borderRadius: "6px", zIndex: 100, overflow: "hidden",
            boxShadow: "0 -4px 16px rgba(0,0,0,0.4)",
            maxHeight: "180px", overflowY: "auto",
          }}>
            <div style={{
              padding: "4px 8px 2px", fontSize: "9px", letterSpacing: "0.08em",
              color: mentionSigil === "@" ? "var(--accent)" : "var(--ai)",
              fontFamily: "var(--font-ui)", fontWeight: 600, textTransform: "uppercase",
            }}>
              {mentionSigil === "@" ? "entities & components" : "project files"}
            </div>
            {filteredCandidates.slice(0, 12).map((c, i) => (
              <button
                key={c.insert}
                onMouseDown={e => { e.preventDefault(); insertMention(c); }}
                onMouseEnter={() => setMentionIdx(i)}
                style={{
                  display: "flex", alignItems: "center", gap: "8px",
                  width: "100%", textAlign: "left",
                  padding: "4px 10px",
                  background: i === mentionIdx ? (mentionSigil === "@" ? "var(--accent-glow)" : "var(--ai-glow)") : "none",
                  border: "none",
                  borderLeft: `2px solid ${i === mentionIdx ? (mentionSigil === "@" ? "var(--accent)" : "var(--ai)") : "transparent"}`,
                  color: i === mentionIdx ? (mentionSigil === "@" ? "var(--accent)" : "var(--ai)") : "var(--text-bright)",
                  fontFamily: "var(--font-mono)", fontSize: "11px", cursor: "pointer",
                }}
              >
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {c.label}
                </span>
                {c.hint && (
                  <span style={{ fontSize: "9px", color: "var(--text-dim)", flexShrink: 0 }}>
                    {c.hint}
                  </span>
                )}
              </button>
            ))}
          </div>
        )}

        <div style={{
          background: "var(--bg-2)",
          border: `1px solid var(--border-bright)`,
          borderRadius: "6px",
          overflow: "hidden",
        }}>
          <textarea
            ref={textareaRef}
            value={input}
            onChange={handleInputChange}
            onKeyDown={e => {
              if (mentionQuery !== null && filteredCandidates.length > 0) {
                if (e.key === "ArrowDown") { e.preventDefault(); setMentionIdx(i => Math.min(i + 1, filteredCandidates.length - 1)); return; }
                if (e.key === "ArrowUp")   { e.preventDefault(); setMentionIdx(i => Math.max(i - 1, 0)); return; }
                if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); insertMention(filteredCandidates[mentionIdx]); return; }
                if (e.key === "Escape")    { setMentionQuery(null); return; }
              }
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                sendMessage();
              }
            }}
            placeholder="Ask about your scene… (type @ to mention an entity)"
            rows={3}
            style={{
              width: "100%",
              background: "transparent",
              border: "none",
              outline: "none",
              resize: "none",
              fontFamily: "var(--font-mono)",
              fontSize: "11px",
              color: "var(--text-bright)",
              padding: "8px 8px 4px",
              lineHeight: 1.5,
            }}
          />
          <div style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "4px 8px",
          }}>
            <span style={{ fontSize: "10px", color: "var(--text-dim)" }}>
              ↑ {[...activeFlags].join(" · ")} in context
            </span>
            <button
              onClick={sendMessage}
              disabled={thinking || !input.trim()}
              style={{
                background: thinking || !input.trim() ? "var(--bg-3)" : "var(--ai)",
                border: "none",
                borderRadius: "var(--radius)",
                color: thinking || !input.trim() ? "var(--text-muted)" : "#fff",
                fontFamily: "var(--font-ui)",
                fontWeight: 700,
                fontSize: "11px",
                padding: "4px 10px",
                cursor: thinking || !input.trim() ? "default" : "pointer",
                letterSpacing: "0.05em",
              }}
            >↵ SEND</button>
          </div>
        </div>
      </div>

      <style>{`
        @keyframes bounce {
          0%, 100% { transform: translateY(0); }
          50% { transform: translateY(-4px); }
        }
      `}</style>
    </div>
  );
}

// ─── Markdown renderer ───────────────────────────────────────────────────────

function renderInline(text: string): React.ReactNode[] {
  // Process **bold**, *italic*, `code`, and @mentions in a single pass
  const parts: React.ReactNode[] = [];
  const re = /(\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`|(@\w+\(id:\d+\)))/g;
  let last = 0, key = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) parts.push(text.slice(last, m.index));
    if (m[2] !== undefined) parts.push(<strong key={key++} style={{ color: "var(--text-white)", fontWeight: 700 }}>{m[2]}</strong>);
    else if (m[3] !== undefined) parts.push(<em key={key++} style={{ color: "var(--text-bright)", fontStyle: "italic" }}>{m[3]}</em>);
    else if (m[4] !== undefined) parts.push(<code key={key++} style={{ fontFamily: "var(--font-mono)", fontSize: "10px", background: "var(--bg-0)", padding: "1px 4px", borderRadius: "3px", color: "var(--accent)" }}>{m[4]}</code>);
    else if (m[5] !== undefined) parts.push(<span key={key++} style={{ color: "var(--accent)", fontFamily: "var(--font-mono)", fontSize: "10px" }}>{m[5]}</span>);
    last = m.index + m[0].length;
  }
  if (last < text.length) parts.push(text.slice(last));
  return parts;
}

function Markdown({ text }: { text: string }) {
  const lines = text.split("\n");
  const nodes: React.ReactNode[] = [];
  let listItems: string[] = [];
  let codeLines: string[] = [];
  let codeLang = "";
  let inCode = false;
  let key = 0;

  const flushList = () => {
    if (listItems.length === 0) return;
    nodes.push(
      <ul key={key++} style={{ margin: "4px 0 4px 14px", padding: 0, listStyle: "disc" }}>
        {listItems.map((li, i) => (
          <li key={i} style={{ marginBottom: "1px" }}>{renderInline(li)}</li>
        ))}
      </ul>
    );
    listItems = [];
  };

  const flushCode = () => {
    if (codeLines.length === 0) { inCode = false; return; }
    // Trim leading/trailing blank lines from the block
    while (codeLines.length && codeLines[0].trim() === "") codeLines.shift();
    while (codeLines.length && codeLines[codeLines.length - 1].trim() === "") codeLines.pop();
    nodes.push(
      <div key={key++} style={{ margin: "4px 0", borderRadius: "4px", overflow: "hidden", border: "1px solid var(--border)" }}>
        {codeLang && (
          <div style={{ padding: "2px 8px", background: "var(--bg-1)", fontSize: "9px", color: "var(--text-dim)", letterSpacing: "0.08em", fontFamily: "var(--font-mono)", borderBottom: "1px solid var(--border)" }}>
            {codeLang}
          </div>
        )}
        <pre style={{ margin: 0, padding: "6px 8px", background: "var(--bg-0)", fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--text-bright)", overflowX: "auto", lineHeight: 1.5, whiteSpace: "pre" }}>
          {codeLines.join("\n")}
        </pre>
      </div>
    );
    codeLines = [];
    codeLang = "";
    inCode = false;
  };

  for (const raw of lines) {
    const line = raw.trimEnd();

    // Code fence toggle
    const fenceMatch = line.match(/^```(\w*)/);
    if (fenceMatch) {
      if (inCode) {
        flushCode();
      } else {
        flushList();
        inCode = true;
        codeLang = fenceMatch[1] || "";
      }
      continue;
    }

    if (inCode) {
      codeLines.push(raw); // preserve raw indentation inside code blocks
      continue;
    }

    // Heading: ### or ## or #
    const hMatch = line.match(/^(#{1,3})\s+(.+)/);
    if (hMatch) {
      flushList();
      const level = hMatch[1].length;
      const sizes = ["13px", "12px", "11px"];
      nodes.push(
        <div key={key++} style={{ fontSize: sizes[level - 1], fontWeight: 700, color: "var(--text-white)", marginTop: level === 1 ? "6px" : "4px", marginBottom: "2px" }}>
          {renderInline(hMatch[2])}
        </div>
      );
      continue;
    }

    // List item: - or *
    const liMatch = line.match(/^[-*]\s+(.*)/);
    if (liMatch) {
      listItems.push(liMatch[1]);
      continue;
    }

    // Horizontal rule
    if (/^---+$/.test(line.trim())) {
      flushList();
      nodes.push(<hr key={key++} style={{ border: "none", borderTop: "1px solid var(--border)", margin: "6px 0" }} />);
      continue;
    }

    // Blank line
    if (line.trim() === "") {
      flushList();
      nodes.push(<div key={key++} style={{ height: "4px" }} />);
      continue;
    }

    flushList();
    nodes.push(<div key={key++}>{renderInline(line)}</div>);
  }

  flushList();
  if (inCode) flushCode(); // unclosed fence — render anyway
  return <div style={{ display: "flex", flexDirection: "column", gap: "1px" }}>{nodes}</div>;
}

// ─── Action display ───────────────────────────────────────────────────────────

function entityRef(a: Record<string, unknown>): string {
  if (a.entity_name) return `"${a.entity_name}"`;
  if (a.entity_id != null) return `#${a.entity_id}`;
  return "?";
}

function ActionBody({ action }: { action: AiAction }) {
  const a = action as Record<string, unknown>;
  switch (action.type) {
    case "suggest_fix":      return <>{String(a.description ?? "")}</>;
    case "edit_transform":   return <>Entity {entityRef(a)}</>;
    case "create_entity":    return <>Name: {String(a.name ?? "")}</>;
    case "delete_entity":    return <>Entity {entityRef(a)}</>;
    case "rename_entity":    return <>Entity {entityRef(a)} → {String(a.name ?? "")}</>;
    case "write_script":     return <>Path: {String(a.path ?? "")}</>;
    case "attach_script":    return <>Entity {entityRef(a)} ← {String(a.path ?? "")}</>;
    case "add_component":    return <>Entity {entityRef(a)} + {String(a.component_type ?? "")}</>;
    case "remove_component": return <>Entity {entityRef(a)} − {String(a.component_type ?? "")}</>;
    case "patch_component":  return <>Entity {entityRef(a)} [{String(a.component_idx ?? "?")}]</>;
    default: return <>{action.type}</>;
  }
}

interface ActionCardProps {
  action: AiAction;
  status: ActionStatus;
  onApply: () => void;
}

const STATUS_STYLE: Record<ActionStatus, { bg: string; border: string; color: string; label: string }> = {
  pending:    { bg: "rgba(255,200,50,0.08)",  border: "rgba(255,200,50,0.3)",  color: "#c8a030", label: "pending"     },
  ok:         { bg: "rgba(78,203,138,0.10)",  border: "var(--green)",          color: "var(--green)", label: "applied" },
  failed:     { bg: "rgba(220,60,60,0.10)",   border: "rgba(220,60,60,0.5)",   color: "#e05555", label: "failed"      },
  suggestion: { bg: "var(--ai-glow)",         border: "var(--ai-dim)",         color: "var(--ai)", label: "suggestion" },
};

function ActionCard({ action, status, onApply }: ActionCardProps) {
  const isSuggestion = action.type === "suggest_fix";
  const title: Record<string, string> = {
    edit_transform:   "Edit Transform",
    write_script:     "Write Script",
    attach_script:    "Attach Script",
    create_entity:    "Create Entity",
    delete_entity:    "Delete Entity",
    rename_entity:    "Rename Entity",
    add_component:    "Add Component",
    remove_component: "Remove Component",
    patch_component:  "Patch Component",
    suggest_fix:      "Suggestion",
  };
  const label = title[action.type] ?? action.type;
  const ss = STATUS_STYLE[status] ?? STATUS_STYLE.pending;

  return (
    <div style={{
      background: "var(--bg-2)",
      border: `1px solid ${status === "failed" ? "rgba(220,60,60,0.3)" : "var(--border)"}`,
      borderRadius: "var(--radius)",
      overflow: "hidden",
      fontSize: "10px",
    }}>
      <div style={{
        display: "flex",
        alignItems: "center",
        gap: "6px",
        padding: "5px 8px",
        borderBottom: "1px solid var(--border)",
      }}>
        <span style={{ color: "var(--text-bright)", fontFamily: "var(--font-ui)", fontWeight: 600, fontSize: "11px" }}>{label}</span>
        <div style={{ flex: 1 }} />
        <span style={{
          padding: "1px 6px",
          borderRadius: "var(--radius)",
          background: ss.bg,
          border: `1px solid ${ss.border}`,
          color: ss.color,
          fontSize: "9px",
        }}>{ss.label}</span>
      </div>

      <div style={{ padding: "6px 8px", color: "var(--text-muted)", lineHeight: 1.5 }}>
        <ActionBody action={action} />
      </div>

      {isSuggestion && (
        <div style={{
          display: "flex",
          gap: "4px",
          padding: "4px 8px",
          borderTop: "1px solid var(--border)",
        }}>
          <button onClick={onApply} style={{
            background: "var(--ai-glow)",
            border: "1px solid var(--ai-dim)",
            borderRadius: "var(--radius)",
            color: "var(--ai)",
            fontFamily: "var(--font-mono)",
            fontSize: "10px",
            padding: "3px 8px",
            cursor: "pointer",
          }}>Apply fix</button>
          <button style={{
            background: "none",
            border: "1px solid var(--border)",
            borderRadius: "var(--radius)",
            color: "var(--text-muted)",
            fontFamily: "var(--font-mono)",
            fontSize: "10px",
            padding: "3px 8px",
            cursor: "pointer",
          }}>Dismiss</button>
        </div>
      )}
    </div>
  );
}
