import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProposalData } from "../App";

interface Props {
  proposal: ProposalData;
  onSceneChange: () => void;
  onClose: () => void;
}

type ChangeStatus = "staged" | "committing" | "accepted" | "reverting" | "rejected" | "failed";

export default function ProposalsLane({ proposal, onSceneChange, onClose }: Props) {
  const [statuses, setStatuses] = useState<Record<string, ChangeStatus>>(
    () => Object.fromEntries(proposal.changes.map(c => [c.id, "staged" as ChangeStatus]))
  );
  const [busy, setBusy] = useState(false);

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
      });
      onSceneChange();
      setStatuses(s => ({ ...s, [changeId]: "accepted" }));
    } catch {
      setStatuses(s => ({ ...s, [changeId]: "failed" }));
    }
    maybeCleanup({ ...statuses, [changeId]: "accepted" });
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
      });
      onSceneChange();
      setStatuses(s => ({ ...s, [changeId]: "rejected" }));
    } catch {
      setStatuses(s => ({ ...s, [changeId]: "failed" }));
    }
    maybeCleanup({ ...statuses, [changeId]: "rejected" });
  };

  const maybeCleanup = (nextStatuses: Record<string, ChangeStatus>) => {
    const allDone = proposal.changes.every(c => {
      const s = nextStatuses[c.id];
      return s === "accepted" || s === "rejected";
    });
    if (allDone) invoke("clear_staged_proposal").catch(() => {});
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

function ChangeBlock({ label, detail, stagedCount, scriptCount, status, onAccept, onReject }: {
  label: string;
  detail: string;
  stagedCount: number;
  scriptCount: number;
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
          color: "var(--ink-3)", lineHeight: 1.45, marginBottom: "10px",
        }}>
          {detail}
        </div>
      )}

      {isPending && (
        <div style={{ display: "flex", gap: "6px" }}>
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
