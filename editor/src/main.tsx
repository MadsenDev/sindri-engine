import React from "react";
import ReactDOM from "react-dom/client";
import "./styles/tokens.css";
import App from "./App";

class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { error: Error | null }
> {
  state = { error: null };
  static getDerivedStateFromError(error: Error) { return { error }; }
  render() {
    const { error } = this.state;
    if (error) return (
      <div style={{
        padding: "32px", fontFamily: "monospace", fontSize: "13px",
        background: "#0d0f14", color: "#e05555", height: "100vh",
        display: "flex", flexDirection: "column", gap: "12px",
      }}>
        <div style={{ color: "#e8a020", fontWeight: 700, fontSize: "15px" }}>
          Sindri crashed — render error
        </div>
        <div>{(error as Error).message}</div>
        <pre style={{ fontSize: "11px", color: "#888", whiteSpace: "pre-wrap" }}>
          {(error as Error).stack}
        </pre>
        <button
          onClick={() => {
            for (const key of Object.keys(localStorage)) {
              if (key === "sindri_chat_history" || key.startsWith("sindri_chat_history:")) {
                localStorage.removeItem(key);
              }
            }
            this.setState({ error: null });
          }}
          style={{
            marginTop: "8px", padding: "8px 16px", background: "#1e2233",
            border: "1px solid #e8a020", borderRadius: "4px",
            color: "#e8a020", cursor: "pointer", width: "fit-content",
          }}
        >
          Clear chat history and recover
        </button>
      </div>
    );
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>
);
