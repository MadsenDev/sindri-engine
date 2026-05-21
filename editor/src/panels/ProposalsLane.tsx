import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProposalData } from "../App";

// ─── Diff ─────────────────────────────────────────────────────────────────────

type DiffLine = { kind: "+" | "-" | " "; text: string };

function computeDiff(oldText: string, newText: string): DiffLine[] {
  const oldLines = oldText === "" ? [] : oldText.split("\n");
  const newLines = newText === "" ? [] : newText.split("\n");
  // Myers-style patience diff via LCS
  const lcs = buildLCS(oldLines, newLines);
  const result: DiffLine[] = [];
  let oi = 0, ni = 0, li = 0;
  while (oi < oldLines.length || ni < newLines.length) {
    if (li < lcs.length && oi === lcs[li][0] && ni === lcs[li][1]) {
      result.push({ kind: " ", text: oldLines[oi] });
      oi++; ni++; li++;
    } else if (ni < newLines.length && (li >= lcs.length || ni < lcs[li][1])) {
      result.push({ kind: "+", text: newLines[ni++] });
    } else {
      result.push({ kind: "-", text: oldLines[oi++] });
    }
  }
  return result;
}

function buildLCS(a: string[], b: string[]): [number, number][] {
  const m = a.length, n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = 1; i <= m; i++)
    for (let j = 1; j <= n; j++)
      dp[i][j] = a[i - 1] === b[j - 1] ? dp[i - 1][j - 1] + 1 : Math.max(dp[i - 1][j], dp[i][j - 1]);
  const result: [number, number][] = [];
  let i = m, j = n;
  while (i > 0 && j > 0) {
    if (a[i - 1] === b[j - 1]) { result.push([i - 1, j - 1]); i--; j--; }
    else if (dp[i - 1][j] > dp[i][j - 1]) i--;
    else j--;
  }
  return result.reverse();
}

function ScriptDiff({ path, oldContent, newContent }: { path: string; oldContent: string; newContent: string }) {
  const [open, setOpen] = useState(false);
  const [fullDiff] = useState(() => computeDiff(oldContent, newContent));
  const diff = open ? fullDiff : [];
  const added = fullDiff.filter(l => l.kind === "+").length;
  const removed = fullDiff.filter(l => l.kind === "-").length;

  return (
    <div style={{ marginTop: "8px" }}>
      <button
        onClick={() => setOpen(o => !o)}
        style={{
          background: "none", border: "1px solid var(--rule-2)",
          color: "var(--ink-3)", fontFamily: "var(--font-mono)", fontSize: "10px",
          padding: "3px 8px", cursor: "pointer", display: "flex", alignItems: "center", gap: "8px",
          width: "100%", textAlign: "left",
        }}
      >
        <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{path}</span>
        {added > 0 && <span style={{ color: "var(--moss)" }}>+{added}</span>}
        {removed > 0 && <span style={{ color: "var(--red, #e06c75)" }}>-{removed}</span>}
        <span style={{ color: "var(--ink-4)" }}>{open ? "▲" : "▼"}</span>
      </button>
      {open && (
        <div style={{
          overflowX: "auto", overflowY: "auto", maxHeight: "320px",
          background: "var(--paper)", borderLeft: "2px solid var(--rule-2)",
          fontFamily: "var(--font-mono)", fontSize: "11px", lineHeight: "1.6",
        }}>
          {diff.map((line, i) => (
            <div
              key={i}
              style={{
                padding: "0 8px",
                background: line.kind === "+" ? "rgba(152,195,121,0.12)" : line.kind === "-" ? "rgba(224,108,117,0.12)" : "transparent",
                color: line.kind === "+" ? "var(--moss)" : line.kind === "-" ? "var(--red, #e06c75)" : "var(--ink-3)",
                whiteSpace: "pre",
                display: "flex", gap: "6px",
              }}
            >
              <span style={{ opacity: 0.5, userSelect: "none", minWidth: "10px" }}>{line.kind}</span>
              <span>{line.text}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

interface Props {
  proposal: ProposalData;
  onSceneChange: () => void;
  onClose: () => void;
  onAllResolved?: () => void;
}

type ChangeStatus = "staged" | "committing" | "accepted" | "reverting" | "rejected" | "failed";

export default function ProposalsLane({ proposal, onSceneChange, onClose, onAllResolved }: Props) {
  const [statuses, setStatuses] = useState<Record<string, ChangeStatus>>(
    () => Object.fromEntries(proposal.changes.map(c => [c.id, "staged" as ChangeStatus]))
  );
  const [busy, setBusy] = useState(false);

  const checkAndClearIfDone = (next: Record<string, ChangeStatus>) => {
    const allDone = proposal.changes.every(c => {
      const s = next[c.id];
      return s === "accepted" || s === "rejected";
    });
    if (allDone) {
      invoke("clear_staged_proposal").catch(() => {});
      onAllResolved?.();
    }
  };

  const acceptChange = async (changeId: string) => {
    const change = proposal.changes.find(c => c.id === changeId);
    if (!change) return;
    setStatuses(s => ({ ...s, [changeId]: "committing" }));
    try {
      await invoke("commit_staged_change", {
        entityIds: change.staged_entity_ids,
        modifiedEntityIds: change.modified_entity_ids,
        scriptPaths: change.new_script_paths,
        scriptBackups: change.script_backups ?? [],
        changeId: changeId,
      });
      onSceneChange();
      setStatuses(s => {
        const next = { ...s, [changeId]: "accepted" as ChangeStatus };
        checkAndClearIfDone(next);
        return next;
      });
    } catch {
      setStatuses(s => ({ ...s, [changeId]: "failed" }));
    }
  };

  const rejectChange = async (changeId: string) => {
    const change = proposal.changes.find(c => c.id === changeId);
    if (!change) return;
    setStatuses(s => ({ ...s, [changeId]: "reverting" }));
    try {
      await invoke("revert_staged_change", {
        entityIds: change.staged_entity_ids,
        modifiedEntityIds: change.modified_entity_ids,
        scriptPaths: change.new_script_paths,
        scriptBackups: change.script_backups ?? [],
        changeId: changeId,
      });
      onSceneChange();
      setStatuses(s => {
        const next = { ...s, [changeId]: "rejected" as ChangeStatus };
        checkAndClearIfDone(next);
        return next;
      });
    } catch {
      setStatuses(s => ({ ...s, [changeId]: "failed" }));
    }
  };

  const acceptAll = async () => {
    setBusy(true);
    for (const change of proposal.changes) {
      if (statuses[change.id] === "staged" || statuses[change.id] === "failed") {
        await acceptChange(change.id);
      }
    }
    setBusy(false);
  };

  const rejectAll = async () => {
    setBusy(true);
    for (const change of proposal.changes) {
      if (statuses[change.id] === "staged" || statuses[change.id] === "failed") {
        await rejectChange(change.id);
      }
    }
    setBusy(false);
  };

  const pendingCount = proposal.changes.filter(c => {
    const s = statuses[c.id];
    return s === "staged" || s === "failed";
  }).length;
  const acceptedCount = proposal.changes.filter(c => statuses[c.id] === "accepted").length;

  return (
    <div style={{
      flex: 1, display: "flex", flexDirection: "column",
      overflow: "hidden", fontFamily: "var(--font-ui)",
    }}>
      {/* Header */}
      <div style={{
        padding: "16px 22px 14px",
        borderBottom: "1px solid var(--rule)",
        display: "flex", alignItems: "center", gap: "10px",
        flexShrink: 0,
      }}>
        <span style={{ color: "var(--amber)", fontSize: "15px", lineHeight: 1 }}>✦</span>
        <span style={{
          fontFamily: "var(--font-ui)", fontSize: "13px",
          color: "var(--amber)", flex: 1, fontWeight: 500,
          letterSpacing: "0.06em", textTransform: "uppercase",
        }}>
          Staged Proposal
        </span>
        <button
          onClick={onClose}
          style={{
            background: "none", border: "none", color: "var(--ink-4)",
            cursor: "pointer", fontSize: "18px", lineHeight: 1, padding: 0,
          }}
        >×</button>
      </div>

      {/* Prompt + summary */}
      <div style={{
        padding: "14px 22px",
        borderBottom: "1px solid var(--rule)",
        flexShrink: 0,
      }}>
        <div style={{
          fontFamily: "var(--font-mono)", fontSize: "11px",
          color: "var(--ink-4)", lineHeight: 1.5,
          borderLeft: "2px solid var(--amber)",
          paddingLeft: "10px",
        }}>
          "{proposal.prompt}"
        </div>
        {proposal.summary && (
          <div style={{
            marginTop: "10px",
            fontFamily: "var(--font-ui)", fontSize: "12.5px",
            color: "var(--ink-3)", lineHeight: 1.55,
          }}>
            {proposal.summary}
          </div>
        )}
        <div style={{
          marginTop: "8px",
          fontFamily: "var(--font-mono)", fontSize: "10.5px",
          color: "var(--ink-4)",
        }}>
          Changes are already live in the scene — accept to keep, reject to remove.
        </div>
      </div>

      {/* Changes list */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {proposal.changes.length === 0 ? (
          <div style={{ padding: "22px", color: "var(--ink-3)", fontSize: "13px", lineHeight: 1.5 }}>
            No scene changes in this response.
          </div>
        ) : (
          proposal.changes.map(change => (
            <ChangeBlock
              key={change.id}
              label={change.label}
              detail={change.detail}
              stagedCount={change.staged_entity_ids.length}
              scriptCount={change.new_script_paths.length}
              scriptBackups={change.script_backups}
              scriptNewContents={change.script_new_contents ?? []}
              status={statuses[change.id] ?? "staged"}
              onAccept={() => acceptChange(change.id)}
              onReject={() => rejectChange(change.id)}
            />
          ))
        )}
      </div>

      {/* Footer */}
      {proposal.changes.length > 0 && (
        <div style={{
          borderTop: "1px solid var(--rule)",
          padding: "14px 22px",
          flexShrink: 0,
        }}>
          <div style={{ display: "flex", gap: "8px", marginBottom: "10px" }}>
            <button
              onClick={acceptAll}
              disabled={pendingCount === 0 || busy}
              style={{
                flex: 1, padding: "9px",
                background: pendingCount > 0 && !busy ? "var(--amber)" : "var(--paper-3)",
                border: "none",
                color: pendingCount > 0 && !busy ? "var(--paper)" : "var(--ink-4)",
                fontFamily: "var(--font-ui)", fontSize: "13px",
                cursor: pendingCount > 0 && !busy ? "pointer" : "default",
                fontWeight: 500,
              }}
            >
              {busy ? "Working…" : "Accept all"}
            </button>
            <button
              onClick={rejectAll}
              disabled={pendingCount === 0 || busy}
              style={{
                flex: 1, padding: "9px",
                background: "none",
                border: "1px solid var(--rule-2)",
                color: pendingCount > 0 && !busy ? "var(--ink-3)" : "var(--ink-4)",
                fontFamily: "var(--font-ui)", fontSize: "13px",
                cursor: pendingCount > 0 && !busy ? "pointer" : "default",
              }}
            >Reject all</button>
          </div>
          <div style={{
            fontFamily: "var(--font-mono)", fontSize: "11px",
            color: "var(--ink-4)", textAlign: "center",
          }}>
            {pendingCount > 0
              ? `${pendingCount} change${pendingCount !== 1 ? "s" : ""} staged`
              : acceptedCount > 0
              ? `${acceptedCount} change${acceptedCount !== 1 ? "s" : ""} committed`
              : "all changes reviewed"}
          </div>
        </div>
      )}
    </div>
  );
}

function ChangeBlock({ label, detail, stagedCount, scriptCount, scriptBackups, scriptNewContents, status, onAccept, onReject }: {
  label: string;
  detail: string;
  stagedCount: number;
  scriptCount: number;
  scriptBackups: { path: string; existed: boolean; content: string }[];
  scriptNewContents: { path: string; content: string }[];
  status: ChangeStatus;
  onAccept: () => void;
  onReject: () => void;
}) {
  const isPending = status === "staged" || status === "failed";
  const isAccepted = status === "accepted";
  const isRejected = status === "rejected";
  const isBusy = status === "committing" || status === "reverting";

  return (
    <div style={{
      padding: "14px 22px",
      borderBottom: "1px solid var(--rule)",
      opacity: isRejected ? 0.4 : 1,
      transition: "opacity 0.15s",
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "6px" }}>
        <span style={{
          fontFamily: "var(--font-mono)", fontSize: "10px",
          color: isAccepted ? "var(--moss)" : isRejected ? "var(--ink-4)" : "var(--amber)",
          border: `1px solid ${isAccepted ? "var(--moss)" : isRejected ? "var(--rule-2)" : "var(--amber)"}`,
          padding: "1px 5px",
          whiteSpace: "nowrap",
        }}>
          {isAccepted ? "committed" : isRejected ? "reverted" : isBusy ? (status === "committing" ? "committing…" : "reverting…") : "staged"}
        </span>
        {(stagedCount > 0 || scriptCount > 0) && !isRejected && (
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--ink-4)" }}>
            {[
              stagedCount > 0 ? `${stagedCount} ${stagedCount === 1 ? "entity" : "entities"}` : "",
              scriptCount > 0 ? `${scriptCount} script edit${scriptCount === 1 ? "" : "s"}` : "",
            ].filter(Boolean).join(" · ")}
          </span>
        )}
        {isAccepted && <span style={{ fontSize: "11px", color: "var(--moss)" }}>✓</span>}
      </div>

      <div style={{
        fontFamily: "var(--font-ui)", fontSize: "13px",
        color: "var(--ink)", lineHeight: 1.4, marginBottom: "4px",
      }}>
        {label}
      </div>

      {detail && (
        <div style={{
          fontFamily: "var(--font-ui)", fontSize: "11.5px",
          color: "var(--ink-3)", lineHeight: 1.45, marginBottom: "6px",
        }}>
          {detail}
        </div>
      )}

      {scriptNewContents.map(nc => {
        const backup = scriptBackups.find(b => b.path === nc.path);
        const oldContent = backup?.existed ? backup.content : "";
        return (
          <ScriptDiff key={nc.path} path={nc.path} oldContent={oldContent} newContent={nc.content} />
        );
      })}

      {isPending && (
        <div style={{ display: "flex", gap: "6px", marginTop: "10px" }}>
          <button
            onClick={onAccept}
            style={{
              padding: "4px 14px", background: "none",
              border: "1px solid var(--moss)", color: "var(--moss)",
              fontFamily: "var(--font-mono)", fontSize: "11px",
              cursor: "pointer",
            }}
          >✓ keep</button>
          <button
            onClick={onReject}
            style={{
              padding: "4px 14px", background: "none",
              border: "1px solid var(--rule-2)", color: "var(--ink-4)",
              fontFamily: "var(--font-mono)", fontSize: "11px",
              cursor: "pointer",
            }}
          >✗ remove</button>
          {status === "failed" && (
            <span style={{ fontSize: "11px", color: "var(--red)", fontFamily: "var(--font-mono)", alignSelf: "center" }}>
              failed
            </span>
          )}
        </div>
      )}
    </div>
  );
}
