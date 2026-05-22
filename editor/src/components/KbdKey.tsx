import type React from "react";

export function KbdKey({ children }: { children: React.ReactNode }) {
  return (
    <span style={{
      display: "inline-block", padding: "1px 5px",
      border: "1px solid var(--rule-2)",
      fontFamily: "var(--font-mono)", fontSize: "10.5px",
      color: "var(--ink-4)", margin: "0 1px",
    }}>{children}</span>
  );
}
