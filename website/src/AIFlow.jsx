// AI proposal flow — auto-plays through 4 stages on scroll into view.
// Stages: 1 ask · 2 compose · 3 propose · 4 accept

const FLOW_STAGES = [
  { id: 1, name: "ask",     label: "01 ask"     },
  { id: 2, name: "compose", label: "02 compose" },
  { id: 3, name: "propose", label: "03 propose" },
  { id: 4, name: "accept",  label: "04 accept"  },
];

const PROMPT = "tighten the drone orbit and flag any collider mismatch.";

function AIFlow() {
  const ref = React.useRef(null);
  const seen = useInView(ref, { threshold: 0.25 });
  const [stage, setStage] = React.useState(1);
  const [auto, setAuto] = React.useState(true);

  // Auto-advance once seen
  React.useEffect(() => {
    if (!seen || !auto) return;
    const ms = stage === 1 ? 2800 : stage === 2 ? 2200 : stage === 3 ? 3200 : 4000;
    const t = setTimeout(() => {
      setStage(s => s === 4 ? 1 : s + 1);
    }, ms);
    return () => clearTimeout(t);
  }, [stage, seen, auto]);

  const goto = (i) => { setAuto(false); setStage(i); };

  return (
    <section id="ai" ref={ref} className="ai-flow section">
      <span className="section-label">
        <span className="dot" />
        <span className="num">04</span> · ai · proposal flow
      </span>

      <div className="wrap">
        <div className="section-heading">
          <div className="left">
            <span className="eyebrow"><span className="bar" />the loop · editor workflow</span>
            <h2>Ask. Compose. <span className="amber">Propose.</span> Accept.</h2>
          </div>
          <div className="right">
            <p>
              The proposal protocol maps to the current editor model: real action types for script writes,
              transform edits, and review cards. The animation below is a compact explanation of that workflow,
              not a replacement for the editor chrome.
            </p>
          </div>
        </div>

        <div className="design-target-banner">
          <span className="dot-amber" />
          <span className="label">workflow illustration</span>
          <span>· shows how local Ollama responses become reviewable Sindri actions inside the existing editor model.</span>
        </div>

        <div className="ai-flow-stage">
          <FlowViewport stage={stage} />
          <FlowProposalLane stage={stage} />
        </div>

        <div className="flow-scrubber">
          {FLOW_STAGES.map((s, i) => (
            <button
              key={s.id}
              className={"step" + (stage === s.id ? " active" : "")}
              onClick={() => goto(s.id)}
            >
              {s.label}
            </button>
          ))}
          <div className="track">
            <div className="fill" style={{ width: `${((stage - 1) / 3) * 100}%`, transition: "width 320ms ease-out" }} />
            {[0, 33.33, 66.66, 100].map(t => (
              <div key={t} className="tick" style={{ left: `calc(${t}% - 0.5px)` }} />
            ))}
          </div>
          <span>{auto ? "auto" : "manual"} · loop</span>
        </div>
      </div>
    </section>
  );
}

function FlowViewport({ stage }) {
  // beacon at left, drone at right; ghost trail appears in stage 3+
  // in accept, ghost trail becomes solid (moss).
  const droneStart = { x: 480, y: 80 };
  const beacon = { x: 100, y: 220 };

  // path waypoints from drone -> beacon
  const pathPts = [
    { x: 420, y: 110, d: 0.0 },
    { x: 340, y: 150, d: 0.18 },
    { x: 260, y: 180, d: 0.42 },
    { x: 180, y: 210, d: 0.7 },
  ];

  return (
    <div className="ai-flow-vp">
      <div className="grid-bg" />
      <div className="vp-head">
        <div className="tabs">
          <span className="tab active">editor_scene.sindri</span>
          <span className="tab" style={{ color: "var(--ink-4)" }}>drone.lua</span>
          <span className="tab" style={{ color: "var(--ink-4)" }}>player.lua</span>
        </div>
        <span className="meta">
          {stage === 1 && "● ready · 6 entities"}
          {stage === 2 && <span style={{ color: "var(--amber)" }}>● composing 3 changes…</span>}
          {stage === 3 && <span style={{ color: "var(--amber)" }}>● 3 proposals · awaiting review</span>}
          {stage === 4 && <span style={{ color: "var(--moss)" }}>● accepted · drone.lua updated</span>}
        </span>
      </div>

      {/* Beacon */}
      <div className="flow-entity beacon" style={{ left: beacon.x, top: beacon.y }}>
        <span className="label">Beacon · Script</span>
      </div>

      {/* Drone — moves toward beacon in accept stage */}
      <div className="flow-entity drone" style={{
        left: stage === 4 ? (beacon.x + 26) : droneStart.x,
        top:  stage === 4 ? (beacon.y - 10) : droneStart.y,
        transition: "left 1.4s ease-out, top 1.4s ease-out",
      }}>
        <span className="label" style={{ color: stage === 4 ? "var(--moss)" : "var(--ink-3)" }}>
          Drone · {stage === 4 ? "orbit tightened" : "Script"}
        </span>
      </div>

      {/* Ghost trail (proposed) */}
      {stage >= 3 && pathPts.map((p, i) => (
        <div key={i}
          className={"flow-entity ghost-trail " + (stage === 4 ? "" : "show")}
          style={{
            left: p.x, top: p.y,
            opacity: stage === 4 ? 0 : undefined,
            animationDelay: `${i * 0.18}s`,
            transition: "opacity 600ms linear",
            width: 28 - i * 2, height: 28 - i * 2,
          }}
        >
          {i === 1 && stage === 3 && <span className="label">+ smaller orbit · proposed</span>}
        </div>
      ))}

      {/* AI sparkle in upper-left while composing */}
      {stage === 2 && (
        <div style={{
          position: "absolute", top: 50, right: 24,
          color: "var(--amber)",
          fontFamily: "var(--font-mono)",
          fontSize: "var(--t-11)",
          letterSpacing: "var(--track-mid)",
          display: "flex", alignItems: "center", gap: 8,
          padding: "6px 10px",
          border: "1px solid var(--amber-dim)",
          background: "var(--amber-glow)",
        }}>
          <IconSparkSmall size={12} />
          <span>composing</span>
          <span className="ai-dots">
            <span style={{ display: "inline-block", animation: "sk-dot 1.4s ease-in-out infinite" }}>·</span>
            <span style={{ display: "inline-block", animation: "sk-dot 1.4s ease-in-out 0.2s infinite" }}>·</span>
            <span style={{ display: "inline-block", animation: "sk-dot 1.4s ease-in-out 0.4s infinite" }}>·</span>
          </span>
        </div>
      )}

      {/* Stage 1: highlight cmd-k call-to-action floating */}
      {stage === 1 && (
        <div style={{
          position: "absolute", left: "50%", top: "55%",
          transform: "translate(-50%, -50%)",
          width: 360, maxWidth: "80%",
          background: "var(--paper-2)",
          border: "1px solid var(--amber-dim)",
          padding: 0,
          animation: "sk-fade-up 240ms ease-out",
        }}>
          <div style={{
            padding: "8px 12px",
            borderBottom: "1px solid var(--rule)",
            fontFamily: "var(--font-display)",
            fontSize: "var(--t-10)",
            letterSpacing: "var(--track-wide)",
            textTransform: "uppercase",
            color: "var(--amber)",
            display: "flex", alignItems: "center", gap: 8,
          }}>
            <IconSparkle size={11} /> ask sindri · ⌘k
          </div>
          <div style={{
            padding: 14,
            fontFamily: "var(--font-sans)",
            fontSize: "var(--t-13)",
            color: "var(--ink)",
            display: "flex", alignItems: "center", gap: 10,
          }}>
            <IconSparkSmall size={13} style={{ color: "var(--amber)" }} />
            <span>{PROMPT}</span>
            <span style={{
              display: "inline-block", width: 7, height: 14,
              background: "var(--amber)",
              animation: "sk-blink 1s steps(1) infinite",
            }} />
          </div>
        </div>
      )}
    </div>
  );
}

function FlowProposalLane({ stage }) {
  return (
    <div className="proposal-lane">
      <div className="lane-head">
        <IconSparkSmall size={12} style={{ color: "var(--amber)" }} />
        <span className="title">ai proposal · awaiting review</span>
        <span className="pending">
          {stage === 1 && "idle"}
          {stage === 2 && "composing…"}
          {stage === 3 && "3 changes"}
          {stage === 4 && <span style={{ color: "var(--moss)" }}>accepted</span>}
        </span>
      </div>

      <div className="prompt">
        <span className="you-tag">you · 14:22</span>
        {PROMPT}
      </div>

      <div className="proposal-changes">
        <ChangeRow
          kind="write_script"
          file="scripts/drone.lua"
          title={<><span className="amber">+</span> Change Drone orbit radius in <code style={{color:"inherit"}}>on_update()</code></>}
          state={stage}
          diff={[
            { t: "ctx", mark: " ", text: "local angle = 0.0" },
            { t: "rem", mark: "-", text: "local radius = 150.0" },
            { t: "add", mark: "+", text: "local radius = 96.0" },
            { t: "ctx", mark: " ", text: "function on_update(self, dt)" },
            { t: "ctx", mark: " ", text: "  transform:set_position(vec2(x, y))" },
            { t: "ctx", mark: " ", text: "end" },
          ]}
        />
        <ChangeRow
          kind="edit_transform"
          file="scenes/editor_scene.sindri"
          title={<><span className="amber">+</span> Reset Drone scale to match its sprite</>}
          state={stage}
          kv={[
            ["entity_id", "2"],
            ["scale_x",   "1.0"],
            ["scale_y",   "1.0"],
          ]}
        />
        <ChangeRow
          kind="suggest_fix"
          file="scenes/editor_scene.sindri"
          title={<><span className="amber">+</span> Review Drone collider dimensions</>}
          state={stage}
          kv={[
            ["entity_id", "2"],
            ["sprite",    "36 × 36"],
            ["collider",  "147 × 145"],
          ]}
        />
      </div>

      <div className="proposal-foot">
        <span className="left">
          {stage <= 2 && "ollama · qwen2.5-vl:7b · local"}
          {stage === 3 && "review · ↑↓ to step · ↵ accept"}
          {stage === 4 && <span style={{ color: "var(--moss)" }}>● applied · 3 files · undo: ⌘z</span>}
        </span>
        <button className="btn-mini">
          <IconRefine size={11} /> Refine
        </button>
        <button className="btn-mini">Reject</button>
        <button className="btn-mini primary">
          <IconCheck size={11} /> Accept all
        </button>
      </div>
    </div>
  );
}

function ChangeRow({ kind, file, title, state, diff, kv }) {
  const accepted = state === 4;
  const active = state === 3;
  return (
    <div className={"change-row" + (active ? " active" : "") + (accepted ? " accepted" : "")}>
      <div className="head">
        <span className={"badge ai"}>{kind}</span>
        <span>{file}</span>
        <span style={{ marginLeft: "auto" }}>
          {accepted && <span style={{ color: "var(--moss)" }}>✓ applied</span>}
        </span>
      </div>
      <div className="title">{title}</div>
      {diff && state >= 2 && (
        <div className="diff">
          {diff.map((d, i) => (
            <div key={i} className={"line " + d.t + (accepted && d.t === "add" ? " accepted" : "")}>
              <span className="mark">{d.mark}</span>
              <span>{d.text}</span>
            </div>
          ))}
        </div>
      )}
      {kv && state >= 2 && (
        <div className="diff" style={{ padding: "4px 0" }}>
          {kv.map(([k, v]) => (
            <div key={k} className={"line " + (accepted ? "add accepted" : "add")}>
              <span className="mark">+</span>
              <span>
                <span style={{ color: "var(--ink-3)" }}>{k}</span>
                <span style={{ color: "var(--ink-4)" }}>{"  = "}</span>
                <span>{v}</span>
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

Object.assign(window, { AIFlow });
