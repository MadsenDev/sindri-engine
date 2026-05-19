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
  runtimeErrors: string[];
  onSceneChange: () => void;
}

type ContextFlag = "scene" | "script" | "viewport" | "errors";

interface MentionCandidate {
  label: string;
  insert: string;
  hint?: string;    // secondary info shown in the popup
}

function extractReferencedScriptPath(message: string, projectFiles: ProjectFile[]): string | null {
  const matches = Array.from(message.matchAll(/#([^\s]+)/g), m => m[1]);
  for (const referenced of matches) {
    const normalized = referenced.trim();
    const hit = projectFiles.find(
      file => file.path === normalized && file.kind === "script"
    );
    if (hit) return hit.path;
  }
  return null;
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

export default function AIChat({ scene, projectPath, openScript, selectedModel, projectFiles, runtimeErrors, onSceneChange }: Props) {
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
    const referencedScriptPath = extractReferencedScriptPath(message, projectFiles);
    const shouldIncludeScript = activeFlags.has("script") || referencedScriptPath !== null;
    let scriptContext = shouldIncludeScript ? openScript : null;

    if (referencedScriptPath && referencedScriptPath !== openScript?.path) {
      try {
        const content = await invoke<string>("get_script", { path: referencedScriptPath });
        scriptContext = { path: referencedScriptPath, content };
      } catch {
        scriptContext = null;
      }
    }

    const history = currentMessages.flatMap(m => {
      if (m.role === "user" && !m.isErrorReport) return [{ role: "user", content: m.text }];
      if (m.role === "ai")                        return [{ role: "assistant", content: m.text }];
      return [];
    });

    const resp = await invoke<{ text: string; actions: AiAction[] }>("send_ai_message", {
      message,
      contextFlags: {
        includeScene: activeFlags.has("scene"),
        includeScript: shouldIncludeScript,
        includeViewport: activeFlags.has("viewport"),
        includeErrors: activeFlags.has("errors"),
      },
      model: selectedModel ?? undefined,
      history,
      openScript: scriptContext,
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
    <div style={{ display: "flex", flexDirection: "column", minHeight: 0, borderTop: "1px solid var(--rule)", background: "var(--paper)" }}>
      {/* Header */}
      <div style={{
        height: "36px",
        borderBottom: "1px solid var(--rule)",
        display: "flex", alignItems: "center",
        padding: "0 16px", gap: "8px", flexShrink: 0,
      }}>
        <span style={{
          fontFamily: "var(--font-ui)", fontSize: "12px",
          color: "var(--amber)", display: "flex", alignItems: "center", gap: "6px",
        }}>
          ✦ AI Chat
        </span>
        <div style={{ flex: 1 }} />
        {/* Context flags */}
        {flags.map(flag => {
          const active = activeFlags.has(flag);
          return (
            <button key={flag} onClick={() => toggleFlag(flag)} style={{
              background: active ? "rgba(240,192,80,0.12)" : "transparent",
              border: `1px solid ${active ? "var(--amber)" : "var(--rule-2)"}`,
              color: active ? "var(--amber)" : "var(--ink-4)",
              fontFamily: "var(--font-mono)", fontSize: "10px",
              padding: "2px 6px", cursor: "pointer",
            }}>{flag}</button>
          );
        })}
        {messages.length > 0 && (
          <button
            onClick={() => { setMessages([]); localStorage.removeItem(chatHistoryKey(projectPath)); }}
            title="Clear history"
            style={{
              background: "none", border: "none", color: "var(--ink-4)",
              cursor: "pointer", fontSize: "12px", padding: "2px 4px",
              fontFamily: "var(--font-mono)",
            }}
          >✕</button>
        )}
      </div>

      {runtimeErrors.length > 0 && (
        <div style={{
          borderBottom: "1px solid var(--rule)",
          background: "rgba(224,85,85,0.08)",
          padding: "6px 16px",
          fontSize: "11px", color: "var(--red)",
          lineHeight: 1.4, flexShrink: 0,
          fontFamily: "var(--font-mono)",
        }}>
          {runtimeErrors.length} runtime error{runtimeErrors.length !== 1 ? "s" : ""}
        </div>
      )}

      {/* Messages */}
      <div style={{
        flex: 1, overflow: "auto", maxHeight: "240px",
        padding: "8px 0",
        display: "flex", flexDirection: "column",
      }}>
        {messages.map((msg, i) => (
          <div key={i} style={{
            padding: "8px 16px",
            borderBottom: "1px solid var(--rule)",
          }}>
            <span style={{
              fontSize: "10px", color: "var(--ink-4)",
              fontFamily: "var(--font-mono)", display: "block", marginBottom: "3px",
            }}>
              {msg.role === "user" ? "you" : "✦ sindri"}
            </span>
            <div style={{
              fontSize: "12.5px",
              color: msg.role === "ai" ? "var(--ink)" : "var(--ink-2)",
              fontFamily: "var(--font-ui)", lineHeight: 1.55,
              wordBreak: "break-word",
            }}>
              {msg.role === "ai" ? <Markdown text={msg.text} /> : msg.text}
            </div>
            {msg.actions && msg.actions.length > 0 && (
              <div style={{ marginTop: "8px", display: "flex", flexWrap: "wrap", gap: "6px" }}>
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
          <div style={{ padding: "12px 16px", display: "flex", alignItems: "center", gap: "10px" }}>
            <span style={{ fontSize: "10px", color: "var(--ink-4)", fontFamily: "var(--font-mono)" }}>✦ sindri</span>
            {[0, 1, 2].map(d => (
              <span key={d} style={{
                width: "6px", height: "6px", background: "var(--amber)", display: "inline-block",
                animation: `v3dot 900ms ${d * 140}ms ease-in-out infinite`,
              }} />
            ))}
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Chat input */}
      <div style={{
        background: "var(--paper-2)",
        borderTop: "1px solid var(--rule)",
        padding: "8px 16px", flexShrink: 0, position: "relative",
      }}>
        {/* Mention popup */}
        {mentionQuery !== null && filteredCandidates.length > 0 && (
          <div style={{
            position: "absolute", bottom: "calc(100% - 4px)", left: "16px", right: "16px",
            background: "var(--paper-2)", border: "1px solid var(--rule-2)",
            zIndex: 100, overflow: "hidden",
            boxShadow: "0 -4px 16px rgba(0,0,0,0.4)",
            maxHeight: "160px", overflowY: "auto",
          }}>
            <div style={{
              padding: "4px 10px 2px", fontSize: "10px",
              color: "var(--amber)", fontFamily: "var(--font-mono)",
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
                  width: "100%", textAlign: "left", padding: "4px 10px",
                  background: i === mentionIdx ? "rgba(240,192,80,0.10)" : "none",
                  border: "none",
                  borderLeft: `2px solid ${i === mentionIdx ? "var(--amber)" : "transparent"}`,
                  color: i === mentionIdx ? "var(--amber)" : "var(--ink-2)",
                  fontFamily: "var(--font-mono)", fontSize: "11px", cursor: "pointer",
                }}
              >
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.label}</span>
                {c.hint && <span style={{ fontSize: "9px", color: "var(--ink-4)", flexShrink: 0 }}>{c.hint}</span>}
              </button>
            ))}
          </div>
        )}

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
            if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendMessage(); }
          }}
          placeholder="Ask about your scene… (@ to mention)"
          rows={2}
          style={{
            width: "100%", background: "var(--paper)",
            border: "1px solid var(--rule-2)", outline: "none", resize: "none",
            fontFamily: "var(--font-ui)", fontSize: "12.5px",
            color: "var(--ink)", padding: "8px 10px", lineHeight: 1.5,
          }}
        />
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          marginTop: "6px",
        }}>
          <span style={{ fontSize: "10.5px", color: "var(--ink-4)", fontFamily: "var(--font-mono)" }}>
            {[...activeFlags].join(" · ")} in context
          </span>
          <button
            onClick={sendMessage}
            disabled={thinking || !input.trim()}
            style={{
              background: thinking || !input.trim() ? "transparent" : "var(--ink)",
              border: "1px solid var(--rule-2)",
              color: thinking || !input.trim() ? "var(--ink-4)" : "var(--paper)",
              fontFamily: "var(--font-ui)", fontSize: "11.5px",
              padding: "4px 12px",
              cursor: thinking || !input.trim() ? "default" : "pointer",
            }}
          >↵ send</button>
        </div>
      </div>
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
    <span style={{
      fontFamily: "var(--font-mono)", fontSize: "10.5px",
      padding: "2px 8px",
      border: `1px solid ${ss.border}`,
      color: ss.color,
      display: "inline-flex", alignItems: "center", gap: "6px",
    }}>
      {label} · {ss.label}
      {isSuggestion && (
        <button onClick={onApply} style={{
          background: "none", border: "none",
          color: "var(--amber)", cursor: "pointer",
          fontFamily: "var(--font-mono)", fontSize: "10px",
          padding: 0, marginLeft: "4px",
        }}>apply</button>
      )}
    </span>
  );
}
