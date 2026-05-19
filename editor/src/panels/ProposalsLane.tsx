import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProposalData } from "../App";

interface Props {
  proposal: ProposalData;
  onSceneChange: () => void;
  onClose: () => void;
  onHoverChange?: (changeId: string | null) => void;
}

type ChangeStatus = "pending" | "applying" | "accepted" | "rejected" | "failed";

export default function ProposalsLane({ proposal, onSceneChange, onClose, onHoverChange }: Props) {
  const [statuses, setStatuses] = useState<Record<string, ChangeStatus>>(
    () => Object.fromEntries(proposal.changes.map(c => [c.id, "pending" as ChangeStatus]))
  );
  const [applyingAll, setApplyingAll] = useState(false);

  const applyChange = async (changeId: string) => {
    const change = proposal.changes.find(c => c.id === changeId);
    if (!change) return;
    setStatuses(s => ({ ...s, [changeId]: "applying" }));
    try {
      await invoke("apply_action", { action: change.action });
      onSceneChange();
      setStatuses(s => ({ ...s, [changeId]: "accepted" }));
    } catch {
      setStatuses(s => ({ ...s, [changeId]: "failed" }));
    }
  };

  const rejectChange = (changeId: string) => {
    setStatuses(s => ({ ...s, [changeId]: "rejected" }));
  };

  const acceptAll = async () => {
    setApplyingAll(true);
    for (const change of proposal.changes) {
      const current = statuses[change.id];
      if (current === "pending" || current === "failed") {
        await applyChange(change.id);
      }
    }
    setApplyingAll(false);
  };

  const rejectAll = () => {
    setStatuses(Object.fromEntries(proposal.changes.map(c => [c.id, "rejected" as ChangeStatus])));
  };

  const pendingCount = proposal.changes.filter(c => statuses[c.id] === "pending" || statuses[c.id] === "failed").length;
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
              actionType={String(change.action.type ?? "")}
              status={statuses[change.id] ?? "pending"}
              onAccept={() => applyChange(change.id)}
              onReject={() => rejectChange(change.id)}
              onMouseEnter={() => onHoverChange?.(change.id)}
              onMouseLeave={() => onHoverChange?.(null)}
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
              disabled={pendingCount === 0 || applyingAll}
              style={{
                flex: 1, padding: "9px",
                background: pendingCount > 0 && !applyingAll ? "var(--amber)" : "var(--paper-3)",
                border: "none",
                color: pendingCount > 0 && !applyingAll ? "var(--paper)" : "var(--ink-4)",
                fontFamily: "var(--font-ui)", fontSize: "13px",
                cursor: pendingCount > 0 && !applyingAll ? "pointer" : "default",
                fontWeight: 500,
              }}
            >
              {applyingAll ? "Applying…" : "Accept all"}
            </button>
            <button
              onClick={rejectAll}
              disabled={pendingCount === 0 || applyingAll}
              style={{
                flex: 1, padding: "9px",
                background: "none",
                border: "1px solid var(--rule-2)",
                color: pendingCount > 0 && !applyingAll ? "var(--ink-3)" : "var(--ink-4)",
                fontFamily: "var(--font-ui)", fontSize: "13px",
                cursor: pendingCount > 0 && !applyingAll ? "pointer" : "default",
              }}
            >Reject all</button>
          </div>
          <div style={{
            fontFamily: "var(--font-mono)", fontSize: "11px",
            color: "var(--ink-4)", textAlign: "center",
          }}>
            {pendingCount > 0
              ? `${pendingCount} change${pendingCount !== 1 ? "s" : ""} pending`
              : acceptedCount > 0
              ? `${acceptedCount} change${acceptedCount !== 1 ? "s" : ""} applied`
              : "all changes reviewed"}
          </div>
        </div>
      )}
    </div>
  );
}

function ChangeBlock({ label, detail, actionType, status, onAccept, onReject, onMouseEnter, onMouseLeave }: {
  label: string;
  detail: string;
  actionType: string;
  status: ChangeStatus;
  onAccept: () => void;
  onReject: () => void;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
}) {
  const isPending = status === "pending" || status === "failed";
  const isAccepted = status === "accepted";
  const isRejected = status === "rejected";
  const isApplying = status === "applying";

  return (
    <div
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      style={{
        padding: "14px 22px",
        borderBottom: "1px solid var(--rule)",
        opacity: isRejected ? 0.4 : 1,
        transition: "opacity 0.15s",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "6px" }}>
        <span style={{
          fontFamily: "var(--font-mono)", fontSize: "10px",
          color: isAccepted ? "var(--moss)" : isRejected ? "var(--ink-4)" : "var(--ink-4)",
          border: `1px solid ${isAccepted ? "var(--moss)" : "var(--rule-2)"}`,
          padding: "1px 5px",
          whiteSpace: "nowrap",
        }}>
          {actionType.replace(/_/g, " ")}
        </span>
        {isAccepted && <span style={{ fontSize: "11px", color: "var(--moss)" }}>✓</span>}
        {isRejected && <span style={{ fontSize: "11px", color: "var(--ink-4)" }}>—</span>}
        {isApplying && <span style={{ fontSize: "11px", color: "var(--amber)" }}>…</span>}
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
          >✓ apply</button>
          <button
            onClick={onReject}
            style={{
              padding: "4px 14px", background: "none",
              border: "1px solid var(--rule-2)", color: "var(--ink-4)",
              fontFamily: "var(--font-mono)", fontSize: "11px",
              cursor: "pointer",
            }}
          >✗ skip</button>
          {status === "failed" && (
            <span style={{ fontSize: "11px", color: "var(--red)", fontFamily: "var(--font-mono)", alignSelf: "center" }}>
              failed — retry?
            </span>
          )}
        </div>
      )}
    </div>
  );
}
